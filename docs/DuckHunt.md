# Duck Hunt Module

A sample module that recreates the classic light-gun game in Arena.

## Gameplay

Players shoot ducks as they fly across the screen. Each round spawns a wave of
ducks with increasing speed and erratic flight patterns. Missing too many
shots ends the round.

## Controls

- **Mouse click** or **Spacebar**: fire the shotgun
- **Arrow keys**: move the crosshair
- **R**: reload when out of shells

## Networking

The server spawns and tracks all ducks. Clients send shot events with their
crosshair position. The server validates hits and broadcasts duck positions
every tick using the standard snapshot protocol. A custom message ID conveys
shot events to the server.

## Assets

- Duck sprite sheet with animation frames
- Background images for sky and ground
- Sound effects for quacks, shots, and reloading
- `module.toml` descriptor under `assets/modules/duck_hunt/`
