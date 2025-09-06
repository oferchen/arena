# Duck Hunt Module

A sample module that recreates the classic light-gun game in Arena.

## Gameplay

Players shoot ducks as they fly across the screen. Ducks follow spline-based
flight paths and are removed immediately when hit thanks to hitscan
weapons. A 90-second round timer counts down; when it expires all remaining
ducks despawn and the round ends. Each hit awards a point that is reflected in
the on-screen HUD.

## Controls

- **Mouse click** or **Spacebar**: fire the shotgun
- **Arrow keys**: move the crosshair
- **R**: reload when out of shells

## Networking

The server spawns and tracks all ducks. State is replicated using the `net`
crate so clients receive up-to-date positions for interpolation. Clients send
shot events with their crosshair position, and the server performs lag
compensation before validating hits. Successful hits result in score updates
that are broadcast to all players.

## Assets

- Duck sprite sheet with animation frames
- Background images for sky and ground
- Sound effects for quacks, shots, and reloading
- `module.toml` descriptor under `assets/modules/duck_hunt/`

## Lifecycle

Entering the module initializes timers, score tracking, and HUD elements. All
resources and entities are cleaned up when the module exits to return the world
to its previous state.
