use gridmath::gridline::GridLine;
use gridmath::*;
use rand::rngs::ThreadRng;
use rand::{RngCore, Rng};
use rayon::prelude::*;
use std::collections::{BinaryHeap, VecDeque};
use std::mem::swap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, AtomicBool};
use crate::particle_set;
use crate::{chunk::*, region::*, collisions::HitInfo, particle::*};

pub const WORLD_WIDTH: i32 = 1440;
pub const WORLD_HEIGHT: i32 = 960;

pub const TRUE_REGION_SIZE: usize = REGION_SIZE as usize * CHUNK_SIZE as usize;

pub trait WorldGenerator {
    fn get_particle(&self, world_pos: GridVec) -> Particle;
}

pub struct World {
    regions: Vec<Region>,
    compressed_regions: Vec<CompressedRegion>,
    loading_regions: VecDeque<LoadingRegion>,
    unloading_regions: VecDeque<UnloadingRegion>,
    generator: Arc<dyn WorldGenerator + Sync + Send>,
    removed_chunks: Vec<GridVec>,
}

pub struct WorldUpdateStats {
    pub chunk_updates: u64,
    pub loaded_regions: usize,
    pub loading_regions: usize,
    pub compressed_regions: usize,
    pub compressing_regions: usize,
    pub region_updates: u64,
}

pub struct WorldUpdateOptions {
    pub force_compress_decompress_all: bool,
}

enum LoadType {
    Generate(Arc<dyn WorldGenerator + Send + Sync>),
    Decompress(Arc<CompressedRegion>),
}

struct LoadingRegion {
    position: GridVec,
    source: LoadType,
    ready: Arc<AtomicBool>,
    region: Arc<Mutex<Option<Region>>>,
}

struct UnloadingRegion {
    position: GridVec,
    region: Arc<Region>,
    ready: Arc<AtomicBool>,
    compressed_region: Arc<Mutex<Option<CompressedRegion>>>,
}

impl World {
    pub fn new(generator: Arc<dyn WorldGenerator + Sync + Send>) -> Self {
        let created: World = World {
            regions: Vec::new(),
            compressed_regions: Vec::new(),
            loading_regions: VecDeque::new(),
            unloading_regions: VecDeque::new(),
            generator,
            removed_chunks: Vec::new(),
        };

        return created;
    }

    fn _add_region_immediate(&mut self, regpos: GridVec) {
        if self.retrieve_region_if_compressed(regpos) {
            return;
        }

        let mut added = Region::new(regpos, self.generator.clone());

        for region in self.regions.iter_mut() {
            region.check_add_neighbor(&mut added);
        }

        self.regions.push(added);
    }

    fn add_region(&mut self, regpos: GridVec) {
        for loader in self.loading_regions.iter() {
            if regpos == loader.position {
                return; // This one has already been requested and we're working on it, cool it
            }
        }

        for unloader in self.unloading_regions.iter() {
            if regpos == unloader.position {
                return; // This one is still being compressed, wait for it to be done so no data is lost
            }
        }

        if self.retrieve_region_if_compressed(regpos) {
            return;
        }

        self.loading_regions.push_back(LoadingRegion::new_generate(regpos, self.generator.clone()));
        self.loading_regions.back_mut().unwrap().start_load();
    }

    fn retrieve_region_if_compressed(&mut self, regpos: GridVec) -> bool {
        let mut to_remove = None;
        let mut index = 0;
        for compreg in self.compressed_regions.iter() {
            if compreg.position == regpos {
                self.loading_regions.push_back(LoadingRegion::new_decompress(regpos, compreg));
                self.loading_regions.back_mut().unwrap().start_load();

                to_remove = Some(index);
                break;
            }
            index += 1;
        }

        if let Some(i) = to_remove {
            self.compressed_regions.remove(i);
            true
        }
        else {
            false
        }
    }

    fn add_loaded_regions_to_sim(&mut self) {
        if let Some(loader) = self.loading_regions.front() {
            if loader.ready.fetch_and(true, std::sync::atomic::Ordering::Relaxed) {
                let loaded = self.loading_regions.pop_front().unwrap();

                if let Ok(add_mutex) = Arc::try_unwrap(loaded.region) {
                    let mut guard = add_mutex.lock().unwrap();
                    if let Some(mut add) = guard.take() {
                        for region in self.regions.iter_mut() {
                            region.check_add_neighbor(&mut add);
                        }
                
                        self.regions.push(add);
                    }
                    
                }
            }
        }
    }

    fn add_unloaded_region_to_list(&mut self) {
        loop {
            if let Some(unloader) = self.unloading_regions.front() {
                if unloader.ready.fetch_and(true, std::sync::atomic::Ordering::Relaxed) {
                    let unloaded = self.unloading_regions.pop_front().unwrap();
    
                    if let Ok(mutex) = Arc::try_unwrap(unloaded.compressed_region) {
                        let mut guard = mutex.lock().unwrap();
    
                        if let Some(reg) = guard.take() {
                            self.compressed_regions.push(reg);
                        }
                    }
                }
                else {
                    break;
                }
            }
            else {
                break;
            }
        }
        
    }

    fn compress_idle_regions(&mut self, visible_bounds: GridBounds, staleness_threshold: u64, force_compress_all: bool) {
        let mut to_remove = Vec::new();
        for region in self.regions.iter_mut() {
            if force_compress_all || (region.staleness > staleness_threshold && !visible_bounds.contains(region.position)) {
                to_remove.push(region.position);
                
                self.removed_chunks.append(&mut region.get_chunk_positions());
                
                let mut reg = Region::new(region.position, self.generator.clone());
                swap(region, &mut reg);
                self.unloading_regions.push_back(UnloadingRegion::new(region.position, reg));
                self.unloading_regions.back_mut().unwrap().start_unload();

                // break; // Only do one region per frame
            }
        }

        for regpos in to_remove.iter() {
            self.remove_region(*regpos);
        }
    }

    fn add_region_if_needed(&mut self, regpos: GridVec) {
        if !self.has_region(regpos) {
            self.add_region(regpos);
        }
    }

    fn remove_region(&mut self, regpos: GridVec) {
        if let Some(index) = self.get_region_index(regpos) {
            self.regions.remove(index);

            for region in self.regions.iter_mut() {
                region.check_remove_neighbor(&regpos);
            }
        }
    }

    fn get_region_index(&self, regpos: GridVec) -> Option<usize> {
        for i in 0..self.regions.len() {
            if self.regions[i].position == regpos {
                return Some(i);
            }
        }
        return None;
    }

    pub fn get_regionpos_for_chunkpos(chunkpos: &GridVec) -> GridVec {
        let mut modpos = chunkpos.clone();
        if modpos.x < 0 {
            modpos.x -= REGION_SIZE as i32 - 1;
        }
        if modpos.y < 0 {
            modpos.y -= REGION_SIZE as i32 - 1;
        }
        GridVec::new(modpos.x / REGION_SIZE as i32, modpos.y / REGION_SIZE as i32)
    }


    fn get_regionpos_for_pos(pos: &GridVec) -> GridVec {
        Self::get_regionpos_for_chunkpos(&Self::get_chunkpos(pos))
    }

    fn has_region(&self, regpos: GridVec) -> bool {
        self.get_region_index(regpos).is_some()
    }

    pub fn contains(&self, pos: GridVec) -> bool {
        for reg in self.regions.iter() {
            if reg.contains_point(&pos) {
                return true;
            }
        }
        return false;
    }

    pub(crate) fn get_chunk_mut(&mut self, chunkpos: &GridVec) -> Option<&mut Box<Chunk>> {
        for reg in self.regions.iter_mut() {
            if reg.contains_chunk(chunkpos) {
                return reg.get_chunk_mut(chunkpos);
            }
        }
        return None;
    }

    pub fn get_chunk(&self, chunkpos: &GridVec) -> Option<&Box<Chunk>> {
        for reg in self.regions.iter() {
            if reg.contains_chunk(chunkpos) {
                return reg.get_chunk(chunkpos);
            }
        }
        return None;
    }

    pub fn get_added_chunks(&mut self) -> Vec<GridVec> {
        let mut set = Vec::new();
        for reg in self.regions.iter_mut() {
            set.append(&mut reg.get_added_chunks());
        }
        return set;
    }

    pub fn reset_updated_chunks(&mut self) {
        for reg in self.regions.iter_mut() {
            reg.clear_updated_chunks();
        }
    }

    pub fn get_updated_chunks(&self) -> Vec<GridVec> {
        let mut set = Vec::new();
        for reg in self.regions.iter() {
            set.append(&mut &mut reg.get_updated_chunks());
        }
        return set;
    }

    pub fn get_removed_chunks(&mut self) -> Vec<GridVec> {
        let set = self.removed_chunks.clone();
        self.removed_chunks.clear();
        return set;
    }

    pub fn get_chunkpos(pos: &GridVec) -> GridVec {
        let mut modpos = pos.clone();
        if modpos.x < 0 {
            modpos.x -= CHUNK_SIZE as i32 - 1;
        }
        if modpos.y < 0 {
            modpos.y -= CHUNK_SIZE as i32 - 1;
        }
        GridVec::new(modpos.x / CHUNK_SIZE as i32, modpos.y / CHUNK_SIZE as i32)
    }

    pub(crate) fn get_chunklocal(pos: GridVec) -> GridVec {
        let mut modded = GridVec::new(pos.x % CHUNK_SIZE as i32, pos.y % CHUNK_SIZE as i32);
        if modded.x < 0 { 
            modded.x += CHUNK_SIZE as i32; 
        }
        if modded.y < 0 { 
            modded.y += CHUNK_SIZE as i32;
        }
        return modded;
    }

    pub fn get_particle(&self, pos: GridVec) -> Particle {
        for reg in self.regions.iter() {
            if reg.contains_point(&pos) {
                return reg.get_particle(pos);
            }
        }

        return Particle::new(ParticleType::Boundary);
    }

    pub fn replace_particle(&mut self, pos: GridVec, new_val: Particle) {
        if !self.contains(pos) {
            let chunkpos = World::get_chunkpos(&pos);
            let regpos = World::get_regionpos_for_chunkpos(&chunkpos);
            self.add_region(regpos);
        }

        let chunkpos = World::get_chunkpos(&pos);
        let chunklocal = World::get_chunklocal(pos);

        if let Some(chunk) = self.get_chunk_mut(&chunkpos) {
            chunk.set_particle(chunklocal.x as u8, chunklocal.y as u8, new_val);
        }
    }
    
    pub fn set_particle_temperature(&mut self, pos: GridVec, temperature: i32, rng: &mut ThreadRng) {
        if !self.contains(pos) {
            let chunkpos = World::get_chunkpos(&pos);
            let regpos = World::get_regionpos_for_chunkpos(&chunkpos);
            self.add_region(regpos);
        }

        let chunkpos = World::get_chunkpos(&pos);
        let chunklocal = World::get_chunklocal(pos);

        if let Some(chunk) = self.get_chunk_mut(&chunkpos) {
            chunk.try_state_change(chunklocal.x as u8, chunklocal.y as u8, temperature, rng);
        }
    }
    
    pub fn replace_particle_filtered(&mut self, pos: GridVec, new_val: Particle, replace_types: ParticleSet) -> Option<ParticleType> {
        if !self.contains(pos) {
            let chunkpos = World::get_chunkpos(&pos);
            let regpos = World::get_regionpos_for_chunkpos(&chunkpos);
            self.add_region(regpos);
        }

        let chunkpos = World::get_chunkpos(&pos);
        let chunklocal = World::get_chunklocal(pos);
        
        if let Some(chunk) = self.get_chunk_mut(&chunkpos) {
            chunk.replace_particle_filtered(chunklocal.x as i16, chunklocal.y as i16, new_val, replace_types)
        }
        else {
            None
        }
    }

    pub fn add_particle(&mut self, pos: GridVec, new_val: Particle) {
        if !self.contains(pos) {
            let chunkpos = World::get_chunkpos(&pos);
            let regpos = World::get_regionpos_for_chunkpos(&chunkpos);
            self.add_region(regpos);
        }

        let chunkpos = World::get_chunkpos(&pos);
        let chunklocal = World::get_chunklocal(pos);

        self.get_chunk_mut(&chunkpos).unwrap().add_particle(chunklocal.x as i16, chunklocal.y as i16, new_val);
    }

    pub fn clear_circle(&mut self, pos: GridVec, radius: i32) {
        self.place_circle(pos, radius, Particle::new(ParticleType::Air), true);
    }

    pub fn place_circle(&mut self, pos: GridVec, radius: i32, new_val: Particle, replace: bool) {
        let left = pos.x - radius;
        let right = pos.x + radius;
        let bottom = pos.y - radius;
        let top = pos.y + radius;

        for y in bottom..top {
            for x in left..right {
                if pos.sq_distance(GridVec{x, y}) < radius.pow(2) {
                    if replace { self.replace_particle(GridVec{x, y}, new_val.clone()); }
                    else { self.add_particle(GridVec{x, y}, new_val.clone()); }
                }
            }
        }
    }
    
    pub fn temp_change_circle(&mut self, pos: GridVec, radius: i32, strength: f64, temperature: i32) {
        let left = pos.x - radius;
        let right = pos.x + radius;
        let bottom = pos.y - radius;
        let top = pos.y + radius;
        
        let mut rng = rand::thread_rng();

        for y in bottom..top {
            for x in left..right {
                if pos.sq_distance(GridVec{x, y}) < radius.pow(2) {
                    let rad_t = f64::sqrt(pos.sq_distance(GridVec{x, y}) as f64) / radius as f64;
                    let local_strength = 0.5 - (rad_t * (0.5 - strength));
                    if rng.gen_bool(local_strength) {
                        self.set_particle_temperature(GridVec{x, y}, temperature, &mut rng)
                    }
                }
            }
        }
    }
    
    pub fn break_circle(&mut self, pos: GridVec, radius: i32, break_strength: f64) {
        let left = pos.x - radius;
        let right = pos.x + radius;
        let bottom = pos.y - radius;
        let top = pos.y + radius;
        
        let mut rng = rand::thread_rng();

        for y in bottom..top {
            for x in left..right {
                if pos.sq_distance(GridVec{x, y}) < radius.pow(2) {
                    let rad_t = 1. - f64::sqrt(pos.sq_distance(GridVec{x, y}) as f64) / radius as f64;
                    let local_strength = (rad_t * break_strength).clamp(0., 1.);
                    if rng.gen_bool(local_strength) {
                        self.replace_particle_filtered(GridVec{x, y}, Particle::new(ParticleType::Gravel), particle_set![ParticleType::Stone]);
                    }
                }
            }
        }
    }
    
    pub fn extract_circle(&mut self, pos: GridVec, radius: i32, filter: ParticleSet) -> Vec<(ParticleType, GridVec)> {
        let mut particles = Vec::new();
        let left = pos.x - radius;
        let right = pos.x + radius;
        let bottom = pos.y - radius;
        let top = pos.y + radius;

        for y in bottom..top {
            for x in left..right {
                let test_pos = GridVec{x, y};
                if pos.sq_distance(test_pos) < radius.pow(2) {
                    let part = self.get_particle(test_pos).particle_type;
                    if filter.test(part) {
                        particles.push((part, test_pos));
                        self.replace_particle(test_pos, Particle::new(ParticleType::Air));
                    }
                }
            }
        }

        particles
    }

    pub fn update(&mut self, visible: GridBounds, target_chunk_updates: u64, update_options: WorldUpdateOptions) -> WorldUpdateStats {
        self.add_loaded_regions_to_sim();
        self.add_unloaded_region_to_list();

        let visible_regions = GridBounds::new_from_extents(
            Self::get_regionpos_for_pos(&visible.bottom_left()),
            Self::get_regionpos_for_pos(&visible.top_right()) + GridVec::new(1, 1)
        );

        for regpos in visible_regions.iter() {
            self.add_region_if_needed(regpos);
        }

        self.compress_idle_regions(visible_regions, 12, update_options.force_compress_decompress_all);

        let max_update_regions = 16;
        let visible_region_count = visible_regions.area();
        let total_visible_priority_boost = 65536;
        let visible_boost_per_region = (total_visible_priority_boost / visible_region_count) as u64;

        let updated_chunk_count = AtomicU64::new(0);
        let updated_region_count = AtomicU64::new(0);

        let mut to_update = Vec::new();
        let mut to_skip = Vec::new();

        let mut estimated_chunk_updates = 0;

        let mut heap = BinaryHeap::with_capacity(self.regions.len());

        for region in self.regions.iter_mut() {
            let up: u64 = region.update_priority + if region.get_bounds().overlaps(visible) { visible_boost_per_region } else { 0 };
            heap.push(RegUpdateInfoWrapper {
                reg: region, priority: up
            });
        }

        while !heap.is_empty() 
            && estimated_chunk_updates < target_chunk_updates 
            && to_update.len() < max_update_regions {
            let reg_wrap = heap.pop().unwrap();
            let region = reg_wrap.reg;

            estimated_chunk_updates += region.last_chunk_updates;
            to_update.push(region);
        }

        for rem_reg in heap.drain() {
            let region = rem_reg.reg;
            to_skip.push(region);
        }
        
        rayon::scope(|s| {
            s.spawn(|_| {
                to_update.par_iter_mut().for_each(|region| {
                    region.commit_updates();
                    updated_region_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                });
            });
            s.spawn(|_| {
                to_skip.par_iter_mut().for_each(|region| {
                    region.skip_update();
                });
            });
        });

        let shift = (rand::thread_rng().next_u32() % 4) as i32;
        for i in 0..4 {
            let phase = i + shift;
            to_update.par_iter_mut().for_each(|region| {
                if region.staleness == 0 {
                    let region_chunk_updates = region.update(phase);
                    updated_chunk_count.fetch_add(region_chunk_updates, std::sync::atomic::Ordering::Relaxed); 
                }
            });
        }

        let chunk_updates = updated_chunk_count.load(std::sync::atomic::Ordering::Relaxed);

        WorldUpdateStats {
            chunk_updates,
            loaded_regions: self.regions.len(),
            loading_regions: self.loading_regions.len(),
            compressed_regions: self.compressed_regions.len(),
            compressing_regions: self.unloading_regions.len(),
            region_updates: updated_region_count.load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    pub fn cast_ray(&self, hitmask: &ParticleSet, line: GridLine) -> Option<HitInfo> {
        let mut last_region = None;

        for worldpos in line.along() {
            let regpos = World::get_regionpos_for_pos(&worldpos);

            if last_region == Some(regpos) {
                continue;
            }
            else {
                last_region = Some(regpos);
                if let Some(index) = self.get_region_index(regpos) {
                    let result = self.regions[index].cast_ray(hitmask, line);
    
                    if result.is_some() {
                        return result;
                    }
                }
            }
        }
        None
    }

    pub fn query_types_in_bounds(&self, bounds: GridBounds) -> ParticleSet {
        let mut types = ParticleSet::none();

        for region in self.regions.iter() {
            if let Some(matches) = region.query_types_in_bounds(bounds) {
                types = types.union(matches);
            }
        }

        types
    }

    pub fn count_matches_in_bounds(&self, bounds: GridBounds, mask: ParticleSet) -> u32 {
        let mut matches = 0;

        for region in self.regions.iter() {
            if let Some(reg_matches) = region.count_matches_in_bounds(bounds, mask) {
                matches += reg_matches;
            }
        }

        matches
    }
}

struct RegUpdateInfoWrapper<'r> {
    reg: &'r mut Region,
    priority: u64,
}

impl Ord for RegUpdateInfoWrapper<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for RegUpdateInfoWrapper<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

impl PartialEq for RegUpdateInfoWrapper<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for RegUpdateInfoWrapper<'_> {

}

impl UnloadingRegion {
    fn new(position: GridVec, region: Region) -> Self { 
        UnloadingRegion {
            position,
            region: Arc::new(region),
            ready: Arc::new(false.into()),
            compressed_region: Arc::new(Mutex::new(None)),
        }
    }

    fn start_unload(&mut self) {
        let ready = self.ready.clone();
        let region = self.region.clone();
        let compressed_region = self.compressed_region.clone();

        rayon::spawn(move || {
            let reg = region.compress_region();
            compressed_region.lock().unwrap().replace(reg);
            ready.store(true, std::sync::atomic::Ordering::Relaxed);
        });
    }
}

impl LoadingRegion {
    fn new_generate(position: GridVec, generator: Arc<dyn WorldGenerator + Send + Sync>) -> Self {
        LoadingRegion {
            position,
            source: LoadType::Generate(generator),
            ready: Arc::new(false.into()),
            region: Arc::new(Mutex::new(None)),
        }
    }

    fn new_decompress(position: GridVec, compressed: &CompressedRegion) -> Self {
        LoadingRegion {
            position,
            source: LoadType::Decompress(Arc::new(compressed.clone())),
            ready: Arc::new(false.into()),
            region: Arc::new(Mutex::new(None)),
        }
    }

    fn start_load(&mut self) {
        let ready = self.ready.clone();
        let region = self.region.clone();
        let position = self.position.clone();

        match &self.source {
            LoadType::Generate(gen) => {
                let generator = gen.clone();
                // println!("Generating region {}", position);

                rayon::spawn(move || {
                    let mut reg = Region::new(position, generator);
                    reg.generate_terrain();
                    region.lock().unwrap().replace(reg);
                    ready.store(true, std::sync::atomic::Ordering::Relaxed);
                });
            }
            LoadType::Decompress(comp) => {
                let compressed = comp.clone();
                // println!("Decompressing region {}", position);

                rayon::spawn(move || {
                    let reg = Region::from_compressed(&compressed);
                    region.lock().unwrap().replace(reg);
                    ready.store(true, std::sync::atomic::Ordering::Relaxed);
                });
            }
        }
    }
}