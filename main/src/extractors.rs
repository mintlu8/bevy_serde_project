use std::{any::type_name, collections::BTreeMap, marker::PhantomData};

use bevy_ecs::{entity::Entity, world::World};
use bevy_hierarchy::{BuildWorldChildren, Children};
use itertools::Itertools;
use crate::{BevyObject, BindBevyObject, BoxError, Error, WorldUtil};

#[allow(unused)]
use bevy_ecs::component::Component;

/// Extractor for casting a [`Component`] to its bound [`BevyObject`].
pub type Object<T> = <T as BindBevyObject>::BevyObject;

/// Extractor that allows a [`BevyObject`] to be missing.
///
/// The underlying data structure is `Option`, 
/// so you can use `#[serde(skip_deserializing_if("Option::is_none"))]`.
pub struct Maybe<T>(PhantomData<T>);

impl<T> BevyObject for Maybe<T> where T: BevyObject {
    type Ser<'t> = Option<T::Ser<'t>> where T: 't;
    type De<'de> = Option<T::De<'de>>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        Ok(Some(T::to_ser(world, entity)?))
    }

    fn from_de(world: &mut World, entity: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        let Some(de) = de else {return Ok(())};
        T::from_de(world, entity, de)?;
        Ok(())
    }
}

/// Extractor for a single [`BevyObject`] in [`Children`]
/// instead of the entity itself. 
///
/// This will iterate through all children
/// to validate uniqueness. [`ChildUnchecked`] is a non-checking
/// alternative. Alternatively use [`ChildVec`] for a list of objects.
///
/// # Errors
///
/// When more than one item is found.
pub struct Child<T>(T);

impl<T> BevyObject for Child<T> where T: BevyObject {
    type Ser<'t> = T::Ser<'t> where T: 't;
    type De<'de> = T::De<'de>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        let Some(children) = world.entity_ok(entity)?.get::<Children>() else {return Ok(None);};
        match children.iter()
            .filter_map(|entity| T::to_ser(world, *entity).transpose())
            .at_most_one() 
        {
            Ok(None) => Ok(None),
            Ok(Some(Ok(item))) => Ok(Some(item)),
            Ok(Some(Err(err))) => Err(err),
            Err(mut iter) => match iter.find_map(Result::err) {
                Some(err) => Err(err),
                None => Err(Error::MoreThenOne { 
                    parent: entity,
                    ty: type_name::<T>()
                }.boxed()),
            }
        }
    }

    fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        let entity = world.spawn(()).id();
        T::from_de(world, entity, de)?;
        world.entity_mut(parent).add_child(entity);
        Ok(())
    }
}

/// Extractor for a single [`BevyObject`] in [`Children`]
/// instead of the entity itself. 
///
/// This will find the first item and
/// may discard duplicate entities. 
/// Alternatively use [`ChildVec`] for a list of objects.
pub struct ChildUnchecked<T>(T);

impl<T> BevyObject for ChildUnchecked<T> where T: BevyObject {
    type Ser<'t> = T::Ser<'t> where T: 't;
    type De<'de> = T::De<'de>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        let Some(children) = world.entity_ok(entity)?.get::<Children>() else {return Ok(None);};
        match children.iter().find_map(|entity| T::to_ser(world, *entity).transpose()) {
            Some(Ok(result)) => Ok(Some(result)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        let entity = world.spawn(()).id();
        T::from_de(world, entity, de)?;
        world.entity_mut(parent).add_child(entity);
        Ok(())
    }
}

/// Extractor for matching [`BevyObject`]s on a [`Children`].
///
/// The underlying data structure is a [`Vec`], 
/// so you can use `#[serde(skip_serializing_if("Vec::is_empty"))]`.
pub struct ChildVec<T>(PhantomData<T>);


impl<T> BevyObject for ChildVec<T> where T: BevyObject {
    type Ser<'t> = Vec<T::Ser<'t>> where T: 't;
    type De<'de> = Vec<T::De<'de>>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        let Some(children) = world.entity_ok(entity)?.get::<Children>() else {
            return Ok(Some(Vec::new()));
        };
        children.iter()
            .filter_map(|entity| T::to_ser(world, *entity).transpose())
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        for item in de {
            let entity = world.spawn(()).id();
            T::from_de(world, entity, item)?;
            world.entity_mut(parent).add_child(entity);
        }
        Ok(())
    }
}


/// Extractor for matching [`BevyObject`]s on a [`Children`].
/// Unlike [`ChildVec`] this tries to present a map like look 
/// and requires unique keys.
///
/// The underlying data structure is a [`BTreeMap`], 
/// so you can use `#[serde(skip_serializing_if("BTreeMap::is_empty"))]`.
pub struct ChildMap<K, V>(PhantomData<(K, V)>);

impl<K, V> BevyObject for ChildMap<K, V> where 
        K: BevyObject, V: BevyObject, for<'t> K::Ser<'t>:Ord, for<'t> K::De<'t>: Ord  {
    type Ser<'t> = BTreeMap<K::Ser<'t>, V::Ser<'t>> where K: 't, V: 't;
    type De<'de> = BTreeMap<K::De<'de>, V::De<'de>>;

    fn to_ser(world: &World, entity: Entity) -> Result<Option<Self::Ser<'_>>, BoxError> {
        let Some(children) = world.entity_ok(entity)?.get::<Children>() else {
            return Ok(Some(BTreeMap::new()));
        };
        children.iter()
            .filter_map(|entity|Some ((
                K::to_ser(world, *entity).transpose()?, 
                V::to_ser(world, *entity), 
            )))
            .map(|(key, value)| {Ok((
                key?,
                value?.ok_or_else(||Error::KeyNoValue { 
                    key: type_name::<K>(), 
                    value: type_name::<V>(), 
                })?
            ))})
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map(Some)
    }

    fn from_de(world: &mut World, parent: Entity, de: Self::De<'_>) -> Result<(), BoxError> {
        for item in de {
            let entity = world.spawn(()).id();
            K::from_de(world, entity, item.0)?;
            V::from_de(world, entity, item.1)?;
            world.entity_mut(parent).add_child(entity);
        }
        Ok(())
    }
}
