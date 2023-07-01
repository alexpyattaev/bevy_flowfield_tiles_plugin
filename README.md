[![crates.io](https://img.shields.io/crates/v/bevy_flowfield_tiles_plugin)](https://crates.io/crates/bevy_flowfield_tiles_plugin)
[![docs.rs](https://docs.rs/bevy_flowfield_tiles_plugin/badge.svg)](https://docs.rs/bevy_flowfield_tiles_plugin)

# bevy_flowfield_tiles_plugin

Inspired by the work of [Elijah Emerson](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter23_Crowd_Pathfinding_and_Steering_Using_Flow_Field_Tiles.pdf) this is an attempt to implement the data structures and logic required to generate a Flowfield representation of a world which can be used to pathfind a moving actor.

| bevy | bevy_flowfield_tiles_plugin |
|------|-----------------------------|
| 0.11 |  0.1                        |

# Table of Contents

1. [Intro](#intro)
1. [Definitions](#definitions)
1. [Design/Process](#designprocess)

## Intro

Pathfinding in games can take different forms and those forms have certain benefits aligned with the type of game they are being applied to. Generally people run across:

* Way-point Graph - points in space linked together, very strict structure, an actor will move from one way-point to another. Great for games played on a small grid, can be cumbersome when multiple actors are sharing a path which could result in physics collisions
* NavMesh - a walkable surface generated from the topology of meshes in a game world. It allows for a range of dynamic movement within the confines of the mesh
* FlowField Tiles - a means of handling crowd and flocking behaviour by generating a flow field (vector field) describing how an actor flows across a world. A large number of actors can flow in unison to an endpoint while sharing the same pathing data structure

For larger and larger environemnts with an increasing number of pathing actors it may be beneficial to adopt a flow field based approach. FlowField Tiles are complex, it's effectively akin to fluid mechanics, so this is an attempt to bring an agnostic implementation to the [Bevy](https://github.com/bevyengine/bevy/tree/main) game engine.

## Definitions

* Sector - a slice of a game world composed of three 2D arrays called fields (`CostField`, `IntegrationField` and `FlowField`). A game world is effectively represented by a number of Sectors
* CostField - a 2D array describing how difficult it is to path through each cell of the array. It is always present in system memory
* Cost - how difficult/expensive it is to path somewhere, you could also call it <i>weight</i>
* Portal - a navigatable point which links one Sector to another
* IntegrationField - a 2D array which uses the CostField to determine a cumulative cost of reaching the goal/endpoint. This is an ephemeral field - it exists when required to calculate a `FlowField`
* FlowField
* Ordinal - a direction based on traditional compass ordinals: N, NE, E, SE, S, SW, W, NW. Used for neighbour discovery of Sectors/field cells

# Design/Process

To generate a set of navigation `FlowFields` the game world is divided into Sectors indexed by `(column, row)` and each Sector has 3 layers of data: `[CostField, IntegrationField, Flowfield]`. Each layer aids the next in building out a path.

## Sector

For a 3-dimensional world the `x-z` plane defines the number of Sectors used to represent it with a constant called `SECTOR_RESOLUTION`, currently enforced at `10`. This means that a for a `30x30` world there would be `3x3` Sectors representing it. Each Sector has an associated unqiue ID taken as its position: `(column, row)`.

<img src="docs/sectors.png" alt="sectors" width="250"/>

Likewise for a `300x550` world you'll be looking at `30` columns and `55` rows. The advantage of dividing a world into Sectors (as opposed to treating the whole world as a giant `Flowfield`) is that the work in generating a path can be split into multiple operations and only touch certain sectors. Say for the `300x550` world you do treat it as a single set of fields - when calculating a path you could potentially have to calculate the Flowfield values for `165,000` grid cells. Splitting it into sectors may mean that your path only takes you through 20 sectors, thereby only requiring `2,000` `Flowfield` grid cells to be calculated.

## CostField

A `CostField` is an `MxN` 2D array of 8-bit values. The values indicate the `cost` of navigating through that cell of the grid. A value of `1` is the default and indicates the easiest `cost`, and a value of `255` is a special value used to indicate that the grid cell is impassable - this could be used to indicate a wall or obstacle. All other values from `2-254` represent increasing cost, for instance a slope or difficult terrain such as a marsh. The idea is that the pathfinding calculations will favour cells with a smaller value before any others. The `CostField` for every sector exists in memory at all times.

<img src="docs/cost_field.png" alt="cf" width="370"/>

This array is used to generate the `IntegrationField` when requesting a navigatable path.

At runtime the `CostField` is generated for each Sector with the default value - although with the feature `ron` it is possible to load the fields from disk. See the [Usage] section below for details on updating the `CostFields` during an inital pass (i.e when loading a level) and tweaking it during gameplay for a world which dynamically evolves with obstacles (flipping a cell to to a higher cost or impassable `255`).

## Portals

Each Sector has up to 4 boundaries with neighbouring Sectors (2 or 3 when the sector is in a corner or along th edge of the game world). Each boundary can contain Portals which indicate a navigatable point from the current Sector to a neighbour. Portals serve a dual purpose, one of which is to provide responsiveness - `FlowFields` may take time to generate so when an actor needs to move a quick A* pathing query can produce an inital path route based on moving from one Portal to another and they can start moving in the general direction to the goal/target/endpoint. Once the `FlowFields` have been built the actor can switch to using them for granular navigation instead.

The following sectors are located away from any edges of the world which means each boundary can have Portals (the purple cells):

<img src="docs/portals.png" alt="portals" width="400" height="500"/><img src="docs/portals_adj.png" alt="portals" width="400" height="500"/>

A Portal is generated at the midpoint of a boundary - in situations where the `CostField` contains `255` costs along the edge then multiple Portals may be generated at the midpoint of each valid pathable segment along the boundary and this is propagated to neighbouring Sectors so that every Portal has a neighbour buddy (as evident in the right hand Sector above, `S(1, 1)` portal `(9, 1)` allows movement into `S(2, 1)` portal `(0, 1)`, even though `S(2, 1)` has a whole boundary that appears completely pathable).

On a larger scale (but still small) and for the simplist `CostField` available, a `2x2` Sector grid produces predictable boundary Portals.

<img src="docs/sectors_portals.png" alt="sector_portals" width="400" height="400"/>

### Portal Graph

For finding a path from one Sector to another at a Portal level all Sectors and Portals are recorded within a data strucutre known as `PortalGraph`. The [petgraph](https://github.com/petgraph/petgraph) library has been used within this structure to store navigational points and paths between them and it gets built in three stages:

1. For all Portals and Sectors add a graph `node`
2. For each sector create `edges` (pathable routes) to and from each Portal `node` - effectively create internal walkable routes of each sector
3. Create `edges` across the Portal `node` on all sector boundaries (walkable route from one sector to another)

This allows the graph to be queried with a `source` sector and a `target` sector and a list of Portals are returned which can be pathed. When a `CostField` is changed this triggers the regeneration of the sector Portals for the region that `CostField` resides in (and its neighbours to ensure homogenous boundaries) and the graph is updated with any new Portals `nodes` and the old ones are removed. This is a particularly difficult and complicated area as the Sectors, Portals and fields are represented in 2D but the graph is effectively 1D - it's a big long list of `nodes`. To handle identifying a graph `node` from a Sector and field grid cell a special data field exists in `PortalGraph` nicknamed the "translator". It's a way of being able to convert between the graph data structure and the 2D data structure back and forth, so from a grid cell you can find its `node` and from a list of `nodes` (like an A* result) you can find the location of each Portal.

## IntegrationField

An `IntegrationField` is an `MxN` 2D array of 16-bit values. It uses the `CostField` to produce a cumulative cost to reach the end goal/target. It's an ephemeral field, as in it gets built for a required sector and then consumed by the `FlowField` calculation.

When a new route needs to be processed the field is reset to `u16::MAX` and the grid cell containing the goal is set to `0`.

A series of passes are performed from the goal as an expanding wavefront calculating the field values:

1. The valid ordinal neighbours of the goal are determined (North, East, South, West, when not against a boundary)
2. For each ordinal grid cell lookup their `CostField` value
3. Add their cost to the `IntegrationFields` cost of the current cell (at the beginning this is the goal so `0`)
4. Propagate to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their integration cost, and repeat until the entire field is done

This produces a nice diamond-like pattern as the wave expands (the underlying `CostField` is set to `1` here):

<img src="docs/int_field_prop0.png" alt="ifp0" width="300" height="310"/><img src="docs/int_field_prop1.png" alt="ifp1" width="300" height="310"/>
<img src="docs/int_field_prop2.png" alt="ifp2" width="300" height="310"/><img src="docs/int_field_prop3.png" alt="ifp3" width="300" height="310"/>

Now a dimaond-like wave isn't exactly realistic in a 3D world of dynamic movement so at some point it should be replaced, based on various articles out there it seems people adopt the [Eikonal equation](https://en.wikipedia.org/wiki/Eikonal_equation) to create a more spherical wave expanding over the 2D grid space.

When it comes to `CostField` containing impassable markers, `255` as black boxes, they are ignored so the wave flows around those areas:

<img src="docs/int_field_prop_impassable.png" alt="ifpi" width="300" height="310"/>

And when your `CostField` is using a range of values to indicate different areas to traverse, such as a steep hill:

<img src="docs/cost_field_hill.png" alt="cfh" width="300" height="310"/><img src="docs/int_field_prop_hill.png" alt="ifph" width="300" height="310"/>

So this encourages the pathing algorithm around obstacles and expensive areas in your world!

This covers calculating the `IntegrationField` for a single sector containing the goal but of course the actor could be in a sector far away, this is where `Portals` come back into play.

We have a path of `Portals` to get the actor to the desired sector, the `IntegrationField` of the goal sector have been calculated so next we "hop" through the boundary `Portals` working backwards from the goal sector to the actor sector (Portals are denoted as a purple shade) to produce a series of `IntegrationFields` for the chaining Sectors describing the flow movement.

<img src="docs/int_field_sector_to_sector_0.png" alt="ifsts0" width="260" height="310"/><img src="docs/int_field_sector_to_sector_1.png" alt="ifsts1" width="260" height="310"/><img src="docs/int_field_sector_to_sector_2.png" alt="ifsts2" width="260" height="310"/>

As an example for a `30x30` world and goal at `0` with an actor at `A` an `IntegrationField` set interrogating all sector `Portals` may produce a set of fields looking similar to:

<img src="docs/int_field_prop_big_example.png" alt="ifpbe" width="75%"/>

In terms of pathfinding the actor will favour flowing "downhill". From the position of the actor and looking at its neighbours a smalller field value in that sectors `IntegrationField` means a more favourable point for reaching the end goal, going from smaller to smaller values, basically a gradient flowing downhill to the destination.

This is the basis of a Flowfield.

Generating the fields for this path programmatically leads to:

<img src="docs/generated_int_fields.png" alt="gif" width="75%"/>

Notice that we don't bother generating the fields for sectors the actor doesn't need to path thorugh. Also a Portal represents the midpoint of a traversable sector boundary, when generating the field we expand the portal to cover its entire segment - this increases efficiency so that an actor can more directly approach its goal rather than zig-zagging to points.

From the `IntegrationFields` we can now build the final set of fields - `FlowFields`

## FlowField

A `FlowField` is an `MxN` 2D array of 8-bit values built from a Sectors `IntegrationField`. The first 4 bits of the value correspond to one of eight ordinal movement directions an actor can take (and a zero vector when impassable) and the second 4 bits correspond to flags which should be used by a character controller/steering pipeline to follow a path.

The directional bits are defined as:

* `0b0000_0001` - North
* `0b0000_0010` - East
* `0b0000_0100` - South
* `0b0000_1000` - West
* `0b0000_0011` - North-East
* `0b0000_0110` - South-East
* `0b0000_1100` - South-West
* `0b0000_1001` - North-West
* `0b0000_0000` - zero vector, represents impassable cells
* `0b0000_1111` - default on `FlowField` initialisation, is always replaced by other values

The assistant flags are defined as:

* `0b0001_0000` - pathable
* `0b0010_0000` - has line-of-sight to goal, an actor no longer needs to follow the field, it can move in a straight line to the goal. This avoids calculating field values that aren't actually needed (UNIMPLEMENTED)
* `0b0100_0000` - indicates the goal
* `0b1000_0000` - indicates a portal goal leading to the next sector
* `0b0000_0000` - 
* `0b0000_0000` - 
* `0b0000_0000` - 
* `0b0000_0000` - 

So a grid cell in the `FlowField` with a value of `0b0001_0110` means the actor should flow in the South-East direction.

Using the `IntegrationFields` generated before, with an actor in the top right trying to reach the bottom left, we now generate the `FlowFields`:

<img src="docs/generated_flow_fields.png" alt="gff"/>

The thinner porition of each cell icon indicates the direction, the actor runs along the flow lines leading to the goal.

## FlowField Cache

To enable actors to reuse `FlowFields` (thus avoiding repeated calculations) all `FlowFields` get placed into a `FlowFieldCache` with an identification system.

# Usage

## Default

## Custom System Setup and Constraints

## Initialising Data

## Path Request

# Features

* `serde` - enables serlialisation on some data types
* `ron` - enables reading `CostField` from files. NB: fixed-size arrays in `.ron` are written as tuples

# Local Info/Tools

## justfile

The [just](https://github.com/casey/just) command line runner is very useful for running a series of build steps/commands locally.

In particular I like to use it to run a debug build (so the compiler can tell me about overflow errors and things), run all tests, generate documentation, compile the binary and finally run it - all from typing `just r` in a terminal.

## Diagrams

Under `./docs` are a series of puml (plantUML) diagrams.

To generate a diagram setup puml use `just` with `just diagram [diagram_name]`, or to generate all of them `just diagrams`.

## rustfmt.toml

Controls formatting settings. I have a prefernce for using tabs simply because in shared projects individuals have their own preference for indentation depth and so automatic tab resizing can make a code base gentler on the eyes.

## clippy.toml

Currently commented out, as I use clippy more I suspect to customise what it does.

## cliff.toml

[git-cliff](https://github.com/orhun/git-cliff) is a very cool changelog generator which uses the style of [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). To generate a changelog based on what the next tag will be you can run `git cliff --tag v1.0.0 --output CHANGELOG.md`

## flamegraph on windows

```sh
cargo install blondie
# set env:DTRACE to blondie_trace.exe
cargo install flmaegraph
# build the app
cargo build --profile=flamegraph
cargo build
cargo build --release
# then use admin terminal!!!
$env:BEVY_ASSET_ROOT="C:\source\rust\bevy_flowfield_tiles_plugin"
cargo flamegraph --package=bevy_flowfield_tiles_plugin --profile=flamegraph # release mode without stripping
cargo flamegraph --package=bevy_flowfield_tiles_plugin --dev # dev mode
```

# LICENSE

Dual license of MIT and Apache.
