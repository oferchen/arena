use bevy::prelude::*;

#[test]
fn fixed_update_ticks_deterministically() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0));

    #[derive(Resource, Default)]
    struct Counter(u32);
    app.init_resource::<Counter>();

    app.add_systems(FixedUpdate, |mut c: ResMut<Counter>| {
        c.0 += 1;
    });

    for _ in 0..60 {
        let timestep = app.world.resource::<Time<Fixed>>().timestep();
        app.world.resource_mut::<Time<Fixed>>().advance_by(timestep);
        app.world.run_schedule(FixedUpdate);
    }

    assert_eq!(app.world.resource::<Counter>().0, 60);
    let elapsed = app.world.resource::<Time<Fixed>>().elapsed_seconds();
    assert!((elapsed - 1.0).abs() < f32::EPSILON);
}
