use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct VecPool<T> {
    pool: Arc<Mutex<VecDeque<Vec<T>>>>,
    initial_capacity: usize,
    max_size: usize,
}

impl<T> VecPool<T> {
    pub fn new(initial_capacity: usize, max_size: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(VecDeque::new())),
            initial_capacity,
            max_size,
        }
    }

    pub fn get(&self) -> Vec<T> {
        let mut pool = self.pool.lock().unwrap();
        pool.pop_front().unwrap_or_else(|| Vec::with_capacity(self.initial_capacity))
    }

    pub fn put(&self, mut vec: Vec<T>) {
        vec.clear();
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.max_size {
            pool.push_back(vec);
        }
    }

    pub fn size(&self) -> usize {
        self.pool.lock().unwrap().len()
    }
}

impl<T> Clone for VecPool<T> {
    fn clone(&self) -> Self {
        Self {
            pool: Arc::clone(&self.pool),
            initial_capacity: self.initial_capacity,
            max_size: self.max_size,
        }
    }
}

impl<T> Default for VecPool<T> {
    fn default() -> Self {
        Self::new(8, 64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_pool_basic() {
        let pool = VecPool::<i32>::new(4, 2);
        
        let vec1 = pool.get();
        assert_eq!(vec1.capacity(), 4);
        
        let mut vec2 = pool.get();
        vec2.push(1);
        vec2.push(2);
        pool.put(vec2);
        
        let vec3 = pool.get();
        assert!(vec3.is_empty());
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_vec_pool_expansion() {
        let pool = VecPool::<i32>::new(2, 3);
        
        let vec1 = pool.get();
        let vec2 = pool.get();
        let vec3 = pool.get();
        
        pool.put(vec1);
        pool.put(vec2);
        pool.put(vec3);
        
        assert_eq!(pool.size(), 3);
        
        let vec4 = pool.get();
        pool.put(vec4);
        
        assert_eq!(pool.size(), 3);
    }

    #[test]
    fn test_vec_pool_max_size() {
        let pool = VecPool::<i32>::new(2, 2);
        
        let vec1 = pool.get();
        let vec2 = pool.get();
        let vec3 = pool.get();
        
        pool.put(vec1);
        pool.put(vec2);
        assert_eq!(pool.size(), 2);
        
        pool.put(vec3);
        assert_eq!(pool.size(), 2);
    }

    #[test]
    fn test_vec_pool_clone() {
        let pool1 = VecPool::<i32>::new(4, 2);
        let pool2 = pool1.clone();
        
        let vec = pool1.get();
        pool1.put(vec);
        
        assert_eq!(pool2.size(), 1);
    }

    #[test]
    fn test_vec_pool_default() {
        let pool = VecPool::<String>::default();
        let vec = pool.get();
        assert_eq!(vec.capacity(), 8);
    }
}
