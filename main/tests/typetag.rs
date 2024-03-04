use bevy_ecs::{component::Component, world::World};
use bevy_serde_project::{bind_object, SerdeProject, WorldExtension};
use bevy_serde_project::typetagged::{BevyTypeTagged, IntoTypeTagged, TypeTagged};
use postcard::ser_flavors::Flavor;
use serde::{Deserialize, Serialize};
use serde_json::json;

macro_rules! impl_animal {
    ($($ty: ident),*) => {
        $(impl Animal for $ty {
            fn name(&self) -> &'static str {
                stringify!($ty)
            }
            fn as_ser(&self) -> &dyn erased_serde::Serialize {
                self
            }
        }

        impl IntoTypeTagged<Box<dyn Animal>> for $ty {
            fn name() -> impl AsRef<str> {
                stringify!($ty)
            }
            fn into_type_tagged(self) -> Box<dyn Animal> {
                Box::new(self)
            }
        })*
    };
}
macro_rules! boxed_animal {
    ($expr: expr) => {
        {
            let val: Box<dyn Animal> = Box::new($expr);
            val
        }
    }
}
pub trait Animal: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn as_ser(&self) -> &dyn erased_serde::Serialize;
}

impl BevyTypeTagged for Box<dyn Animal> {
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ser()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bird(String);

#[derive(Debug, Serialize, Deserialize)]
pub struct Dog {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Turtle;

impl_animal!(Bird, Dog, Turtle);

#[derive(Component, SerdeProject)]
pub struct AnimalComponent {
    #[serde_project("TypeTagged<Box<dyn Animal>>")]
    animal: Box<dyn Animal>
}

bind_object!(
    #[serde(transparent)]
    AnimalComponent as "Animal" {
        animal => AnimalComponent
    }
);

#[test]
pub fn test() {
    let mut world = World::new();
    world.register_typetag::<Box<dyn Animal>, Bird>();
    world.register_typetag::<Box<dyn Animal>, Dog>();
    world.register_typetag::<Box<dyn Animal>, Turtle>();
    world.spawn(AnimalComponent {
        animal: boxed_animal!(Dog { name: "Rex".to_owned() })
    });
    let value = world.save::<AnimalComponent, _>(serde_json::value::Serializer).unwrap();

    assert_eq!(value, json!([{"animal": {"Dog": {"name": "Rex"}}}]));
    world.spawn(AnimalComponent {
        animal: boxed_animal!(Bird("bevy".to_owned()))
    });
    let value = world.save::<AnimalComponent, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, json!([
        {"animal": {"Dog": {"name": "Rex"}}}, 
        {"animal": {"Bird": "bevy"}}
    ]));
    world.spawn(AnimalComponent {
        animal: boxed_animal!(Turtle)
    });
    let value = world.save::<AnimalComponent, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, json!([
        {"animal": {"Dog": {"name": "Rex"}}}, 
        {"animal": {"Bird": "bevy"}}, 
        {"animal": {"Turtle": null}}, 
    ]));

    let value = world.save::<AnimalComponent, _>(serde_json::value::Serializer).unwrap();

    world.despawn_bound_objects::<AnimalComponent>();
    assert_eq!(world.entities().len(), 0);
    world.load::<AnimalComponent, _>(&value).unwrap();
    assert_eq!(world.entities().len(), 3);

    let value = world.save::<AnimalComponent, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, json!([
        {"animal": {"Dog": {"name": "Rex"}}}, 
        {"animal": {"Bird": "bevy"}}, 
        {"animal": {"Turtle": null}}, 
    ]));

    let mut vec = postcard::Serializer{
        output: postcard::ser_flavors::AllocVec::new(),
    };
    world.save::<AnimalComponent, _>(&mut vec).unwrap();
    let result = vec.output.finalize().unwrap();

    world.despawn_bound_objects::<AnimalComponent>();
    assert_eq!(world.entities().len(), 0);

    let mut de = postcard::Deserializer::from_bytes(&result);
    world.load::<AnimalComponent, _>(&mut de).unwrap();
    assert_eq!(world.entities().len(), 3);

    let value = world.save::<AnimalComponent, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, json!([
        {"animal": {"Dog": {"name": "Rex"}}}, 
        {"animal": {"Bird": "bevy"}}, 
        {"animal": {"Turtle": null}}, 
    ]));
}