use std::marker::PhantomData;

use bevy_asset::{Asset, AssetId, AssetIndex};
use bevy_ecs::system::Resource;
use bevy_reflect::Reflect;
use bevy_utils::HashMap;
use uuid::Uuid;

pub trait AssetBvhCache: Send + Sync + 'static {}

#[derive(Resource, Reflect)]
pub struct AssetsBvhCaches<A: Asset, B: AssetBvhCache> {
    dense_storage: HashMap<u64, B>,
    hash_map: HashMap<Uuid, B>,
    marker: PhantomData<fn() -> A>,
}

impl<A: Asset, B: AssetBvhCache> Default for AssetsBvhCaches<A, B> {
    fn default() -> Self {
        Self {
            dense_storage: Default::default(),
            hash_map: Default::default(),
            marker: Default::default(),
        }
    }
}

impl<A: Asset, B: AssetBvhCache> AssetsBvhCaches<A, B> {
    /// Retrieves a reference to the [`BvhCache`] of the asset with the given `id`, if it exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    #[inline]
    pub fn get(&self, id: impl Into<AssetId<A>>) -> Option<&B> {
        match id.into() {
            AssetId::Index { index, .. } => self.dense_storage.get(&index.to_bits()),
            AssetId::Uuid { uuid } => self.hash_map.get(&uuid),
        }
    }

    /// Retrieves a mutable reference to the [`BvhCache`] of the asset with the given `id`, if it exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    #[inline]
    pub fn get_mut(&mut self, id: impl Into<AssetId<A>>) -> Option<&mut B> {
        let id: AssetId<A> = id.into();
        let result = match id {
            AssetId::Index { index, .. } => self.dense_storage.get_mut(&index.to_bits()),
            AssetId::Uuid { uuid } => self.hash_map.get_mut(&uuid),
        };
        result
    }

    /// Removes (and returns) the [`Asset`] with the given `id`, if it exists.
    /// Note that this supports anything that implements `Into<AssetId<A>>`, which includes [`Handle`] and [`AssetId`].
    pub fn remove(&mut self, id: impl Into<AssetId<A>>) -> Option<B> {
        let id: AssetId<A> = id.into();
        match id {
            AssetId::Index { index, .. } => self.dense_storage.remove(&index.to_bits()),
            AssetId::Uuid { uuid } => self.hash_map.remove(&uuid),
        }
    }

    /// Inserts the given `bvh cache`, identified by the given `id` of the asset. If a `bvh cache` already exists for `id`, it will be replaced.
    pub fn insert(&mut self, id: impl Into<AssetId<A>>, bvh_cache: B) {
        match id.into() {
            AssetId::Index { index, .. } => {
                self.insert_with_index(index, bvh_cache);
            }
            AssetId::Uuid { uuid } => {
                self.insert_with_uuid(uuid, bvh_cache);
            }
        }
    }

    pub(crate) fn insert_with_uuid(&mut self, uuid: Uuid, bvh_cache: B) -> Option<B> {
        let result = self.hash_map.insert(uuid, bvh_cache);
        result
    }

    pub(crate) fn insert_with_index(&mut self, index: AssetIndex, bvh_cache: B) -> Option<B> {
        let result = self.dense_storage.insert(index.to_bits(), bvh_cache);
        result
    }
}
