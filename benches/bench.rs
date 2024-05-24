use bevy_app::App;
use bevy_ecs::{component::Component, world::World};
use bevy_reflect::{Reflect, TypeRegistration, TypeRegistry, TypeRegistryArc};
use bevy_scene::{serde::SceneDeserializer, DynamicScene};
use bevy_serde_lens::{bind_query, InWorld, WorldExtension};
use criterion::{criterion_group, criterion_main, Criterion};
use itertools::izip;
use rand::distributions::{Distribution, Standard};
use rand_derive2::RandGen;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect, RandGen)]
pub struct Character(String);

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect, RandGen)]
pub struct Bio {
    pub clan: String,
    pub age: u32,
    pub height: f32,
    pub hobbies: String,
}

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect, RandGen)]
pub enum Gender {
    Male,
    Female,
    NonBinary,
    Other,
}

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect, RandGen)]
pub struct IsDead(bool);

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect, RandGen)]
pub struct Potion(String);

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect, RandGen)]
pub struct Ability(String);

fn thousand_of<T>() -> Vec<T>
where
    Standard: Distribution<T>,
{
    (0..1000).map(|_| rand::random()).collect()
}

pub fn bench_ser_strings(c: &mut Criterion) {
    let strings = thousand_of::<Character>();
    let mut world = World::new();
    world.spawn_batch(strings.iter().cloned());
    let mut world2 = App::new();
    world2.world.spawn_batch(strings.iter().cloned());
    world2.register_type::<Character>();
    let dynamic_scene = DynamicScene::from_world(&world2.world);
    let mut registry = TypeRegistry::new();
    registry.add_registration(TypeRegistration::of::<Character>());
    let registry = TypeRegistryArc {
        internal: Arc::new(RwLock::new(registry)),
    };
    c.bench_function("postcard_strings_vec", |b| {
        b.iter(|| postcard::to_allocvec(&strings).unwrap());
    });
    c.bench_function("postcard_strings_serde_lens", |b| {
        b.iter(|| postcard::to_allocvec(&world.serialize_lens::<Character>()).unwrap());
    });
    c.bench_function("json_strings_vec", |b| {
        b.iter(|| serde_json::to_string(&strings).unwrap());
    });
    c.bench_function("json_strings_serde_lens", |b| {
        b.iter(|| serde_json::to_string(&world.serialize_lens::<Character>()).unwrap());
    });
    c.bench_function("ron_strings_vec", |b| {
        b.iter(|| ron::to_string(&strings).unwrap());
    });
    c.bench_function("ron_strings_serde_lens", |b| {
        b.iter(|| ron::to_string(&world.serialize_lens::<Character>()).unwrap());
    });
    c.bench_function("ron_from_dynamic_scene", |b| {
        b.iter(|| dynamic_scene.serialize_ron(&registry));
    });
    c.bench_function("ron_construct_dynamic_scene", |b| {
        b.iter(|| DynamicScene::from_world(&world2.world).serialize_ron(&registry));
    });
}

pub fn bench_de_strings(c: &mut Criterion) {
    let strings = thousand_of::<Character>();
    let mut world = World::new();
    let postcard = postcard::to_allocvec(&strings).unwrap();
    let json = serde_json::to_string(&strings).unwrap();
    let ron = ron::to_string(&strings).unwrap();
    let mut registry = TypeRegistry::new();
    registry.add_registration(TypeRegistration::of::<Character>());

    let mut world2 = App::new();
    world2.world.spawn_batch(strings.iter().cloned());
    world2.register_type::<Character>();
    let mut registry2 = TypeRegistry::new();
    registry2.add_registration(TypeRegistration::of::<Character>());
    let registry2 = TypeRegistryArc {
        internal: Arc::new(RwLock::new(registry2)),
    };
    let ron2 = DynamicScene::from_world(&world2.world)
        .serialize_ron(&registry2)
        .unwrap();

    c.bench_function("postcard_strings_de", |b| {
        b.iter(|| {
            world.deserialize_scope(|| {
                let _ = postcard::from_bytes::<InWorld<Character>>(&postcard).unwrap();
            })
        });
    });
    c.bench_function("json_strings_de", |b| {
        b.iter(|| {
            world.deserialize_scope(|| {
                let _ = serde_json::from_str::<InWorld<Character>>(&json).unwrap();
            })
        });
    });
    c.bench_function("ron_strings_de", |b| {
        b.iter(|| {
            world.deserialize_scope(|| {
                let _ = ron::from_str::<InWorld<Character>>(&ron).unwrap();
            })
        });
    });
    c.bench_function("ron_dynamic_scene_strings_de", |b| {
        b.iter(|| {
            world.deserialize_scope(|| {
                use serde::de::DeserializeSeed;
                let mut deserializer = ron::Deserializer::from_str(&ron2).unwrap();
                let _ = SceneDeserializer {
                    type_registry: &registry,
                }
                .deserialize(&mut deserializer);
            })
        });
    });
}

pub fn bench_ser_bio(c: &mut Criterion) {
    let bios = thousand_of::<Bio>();
    let mut world = World::new();
    world.spawn_batch(bios.iter().cloned());
    let mut world2 = App::new();
    world2.world.spawn_batch(bios.iter().cloned());
    world2.register_type::<Bio>();
    let dynamic_scene = DynamicScene::from_world(&world2.world);
    let mut registry = TypeRegistry::new();
    registry.add_registration(TypeRegistration::of::<Bio>());
    let registry = TypeRegistryArc {
        internal: Arc::new(RwLock::new(registry)),
    };
    c.bench_function("postcard_bios_serde_lens", |b| {
        b.iter(|| postcard::to_allocvec(&world.serialize_lens::<Bio>()).unwrap());
    });
    c.bench_function("json_bios_serde_lens", |b| {
        b.iter(|| serde_json::to_string(&world.serialize_lens::<Bio>()).unwrap());
    });
    c.bench_function("ron_bios_serde_lens", |b| {
        b.iter(|| ron::to_string(&world.serialize_lens::<Bio>()).unwrap());
    });
    c.bench_function("ron_bios_from_dynamic_scene", |b| {
        b.iter(|| dynamic_scene.serialize_ron(&registry));
    });
    c.bench_function("ron_bios_construct_dynamic_scene", |b| {
        b.iter(|| DynamicScene::from_world(&world2.world).serialize_ron(&registry));
    });
}

pub fn bench_de_bios(c: &mut Criterion) {
    let strings = thousand_of::<Bio>();
    let mut world = World::new();
    let postcard = postcard::to_allocvec(&strings).unwrap();
    let json = serde_json::to_string(&strings).unwrap();
    let ron = ron::to_string(&strings).unwrap();
    let mut registry = TypeRegistry::new();
    registry.add_registration(TypeRegistration::of::<Bio>());

    let mut world2 = App::new();
    world2.world.spawn_batch(strings.iter().cloned());
    world2.register_type::<Character>();
    let mut registry2 = TypeRegistry::new();
    registry2.add_registration(TypeRegistration::of::<Bio>());
    let registry2 = TypeRegistryArc {
        internal: Arc::new(RwLock::new(registry2)),
    };
    let ron2 = DynamicScene::from_world(&world2.world)
        .serialize_ron(&registry2)
        .unwrap();

    c.bench_function("postcard_bios_de", |b| {
        b.iter(|| {
            world.deserialize_scope(|| {
                let _ = postcard::from_bytes::<InWorld<Bio>>(&postcard).unwrap();
            })
        });
    });
    c.bench_function("json_bios_de", |b| {
        b.iter(|| {
            world.deserialize_scope(|| {
                let _ = serde_json::from_str::<InWorld<Bio>>(&json).unwrap();
            })
        });
    });
    c.bench_function("ron_bios_de", |b| {
        b.iter(|| {
            world.deserialize_scope(|| {
                let _ = ron::from_str::<InWorld<Bio>>(&ron).unwrap();
            })
        });
    });
    c.bench_function("ron_bios_dynamic_scene_de", |b| {
        b.iter(|| {
            world.deserialize_scope(|| {
                use serde::de::DeserializeSeed;
                let mut deserializer = ron::Deserializer::from_str(&ron2).unwrap();
                let _ = SceneDeserializer {
                    type_registry: &registry,
                }
                .deserialize(&mut deserializer);
            })
        });
    });
}

bind_query!(
    pub struct Archetypal {
        character: Character,
        bio: Bio,
        gender: Gender,
        is_dead: IsDead,
    }
);

pub fn bench_ser_archetypal(c: &mut Criterion) {
    let charas = thousand_of::<Character>();
    let bios = thousand_of::<Bio>();
    let gender = thousand_of::<Gender>();
    let dead = thousand_of::<IsDead>();
    let mut world = World::new();
    world.spawn_batch(izip!(
        charas.clone(),
        bios.clone(),
        gender.clone(),
        dead.clone()
    ));
    let mut world2 = App::new();
    world2.world.spawn_batch(izip!(charas, bios, gender, dead));
    world2.register_type::<Character>();
    world2.register_type::<Bio>();
    world2.register_type::<Gender>();
    world2.register_type::<IsDead>();
    let dynamic_scene = DynamicScene::from_world(&world2.world);
    let mut registry = TypeRegistry::new();
    registry.add_registration(TypeRegistration::of::<Character>());
    registry.add_registration(TypeRegistration::of::<Bio>());
    registry.add_registration(TypeRegistration::of::<Gender>());
    registry.add_registration(TypeRegistration::of::<IsDead>());
    let registry = TypeRegistryArc {
        internal: Arc::new(RwLock::new(registry)),
    };
    c.bench_function("postcard_archetypal_serde_lens", |b| {
        b.iter(|| postcard::to_allocvec(&world.serialize_lens::<Archetypal>()).unwrap());
    });
    c.bench_function("json_archetypal_serde_lens", |b| {
        b.iter(|| serde_json::to_string(&world.serialize_lens::<Archetypal>()).unwrap());
    });
    c.bench_function("ron_archetypal_serde_lens", |b| {
        b.iter(|| ron::to_string(&world.serialize_lens::<Archetypal>()).unwrap());
    });
    c.bench_function("ron_archetypal_from_dynamic_scene", |b| {
        b.iter(|| dynamic_scene.serialize_ron(&registry));
    });
    c.bench_function("ron_archetypal_construct_dynamic_scene", |b| {
        b.iter(|| DynamicScene::from_world(&world2.world).serialize_ron(&registry));
    });
}

criterion_group!(
    benches,
    bench_ser_strings,
    bench_de_strings,
    bench_ser_bio,
    bench_de_bios,
    bench_ser_archetypal
);
criterion_main!(benches);

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect)]
pub struct Age(String);

#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect)]
pub struct Stats(String);
