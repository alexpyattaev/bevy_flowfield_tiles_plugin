//! Calculates the [IntegrationField]s from a set of [CostField]s and displays the cell values in a UI grid.
//! 
//! For sectors which an actor does not need to traverse they are not generated or rendered
//!

use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::flowfields::{
	integration_field::IntegrationField, sectors::{SectorCostFields, SectorPortals}, MapDimensions, portal::portal_graph::PortalGraph,
};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_systems(Startup, (setup,))
		.run();
}

fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// calculate the fields
	let map_dimensions = MapDimensions::new(30, 30);
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let sector_cost_fields = SectorCostFields::from_file(path);
	let mut sector_portals = SectorPortals::new(map_dimensions.get_column(), map_dimensions.get_row());
	// update default portals for cost fields
	for (sector_id, _v) in sector_cost_fields.get() {
		sector_portals.update_portals(*sector_id, &sector_cost_fields, map_dimensions.get_column(), map_dimensions.get_row());
	}
	// generate the portal graph
	let portal_graph = PortalGraph::new(&sector_portals, &sector_cost_fields, map_dimensions.get_column(), map_dimensions.get_row());
	//
	let source_sector = (2, 0);
	let source_grid_cell = (7, 3);
	let target_sector = (0, 2);
	let target_grid_cell = (0, 6);
	// path from actor to goal sectors
	let node_path = portal_graph.find_best_path((source_sector, source_grid_cell), (target_sector, target_grid_cell), &sector_portals, &sector_cost_fields).unwrap();
	// convert to grid and sector coords
	let mut path = portal_graph.convert_index_path_to_sector_portal_cells(node_path.1, &sector_portals);
	// original order is from actor to goal, int fields need to be processed the other way around
	path.reverse();
	let mut map = HashMap::new();
	for p in path.iter() {
		if !map.contains_key(&p.0) {
			map.insert(p.0, p.1);
		}
	}
	// prep int fields
	let mut sector_int_fields = BTreeMap::new();
	for (sector_id, goals) in map.iter() {
		let mut int_field = IntegrationField::new(*goals);
		let cost_field = sector_cost_fields.get().get(sector_id).unwrap();
		int_field.calculate_field(*goals, cost_field);
		sector_int_fields.insert(sector_id, int_field);
	}
	// create a UI grid
	cmds.spawn(Camera2dBundle::default());
	cmds.spawn(NodeBundle {
		// background canvas
		style: Style {
			size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
			flex_direction: FlexDirection::Column,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			..Default::default()
		},
		background_color: BackgroundColor(Color::NONE),
		..Default::default()
	})
	.with_children(|p| {
		// a centred box to contain the fields
		p.spawn(NodeBundle {
			style: Style {
				size: Size::new(Val::Px(1000.0), Val::Px(1000.0)),
				flex_direction: FlexDirection::Column,
				flex_wrap: FlexWrap::Wrap,
				flex_shrink: 0.0,
				..Default::default()
			},
			background_color: BackgroundColor(Color::WHITE),
			..Default::default()
		})
		.with_children(|p| {
			// create an area for each sector int field
			for i in 0..map_dimensions.get_column() / 10 {
				for j in 0..map_dimensions.get_row() / 10 {
					// bounding node of a sector
					p.spawn(NodeBundle{
						style: Style {
							size: Size::new(Val::Percent(100.0 / ((map_dimensions.get_column() / 10)) as f32), Val::Percent(100.0 / ((map_dimensions.get_row() / 10)) as f32)),
							flex_direction: FlexDirection::Column,
							flex_wrap: FlexWrap::Wrap,
							flex_shrink: 0.0,
							..Default::default()
						},
						..Default::default()
					}).with_children(|p| {
						// the array area of the sector
						let int_field = sector_int_fields.get(&(i, j));
						match int_field {
							Some(field) => {
								// create each column from the field
								for array in field.get_field().iter() {
									p.spawn(NodeBundle {
										style: Style {
											size: Size::new(Val::Percent(10.0), Val::Percent(100.0)),
											flex_direction: FlexDirection::Column,
											..Default::default()
										},
										..Default::default()
									})
									.with_children(|p| {
										// create each row value of the column
										for value in array.iter() {
											p.spawn(NodeBundle {
												style: Style {
													size: Size::new(Val::Percent(100.0), Val::Percent(10.0)),
													justify_content: JustifyContent::Center,
													align_items: AlignItems::Center,
													..Default::default()
												},
												..Default::default()
											})
											.with_children(|p| {
												p.spawn(TextBundle::from_section(
													value.to_string(),
													TextStyle {
														font: asset_server.load("fonts/FiraSans-Bold.ttf"),
														font_size: 10.0,
														color: Color::BLACK,
													},
												));
											});
										}
									});
								}
							},
							None => {
								// // sectors without int field calculated get an X in each grid cell
								// for _ in 0..10 {
								// 	p.spawn(NodeBundle {
								// 		style: Style {
								// 			size: Size::new(Val::Percent(10.0), Val::Percent(100.0)),
								// 			flex_direction: FlexDirection::Column,
								// 			..Default::default()
								// 		},
								// 		..Default::default()
								// 	})
								// 	.with_children(|p| {
								// 		// create each row value of the column
								// 		for _ in 0..10 {
								// 			p.spawn(NodeBundle {
								// 				style: Style {
								// 					size: Size::new(Val::Percent(100.0), Val::Percent(10.0)),
								// 					justify_content: JustifyContent::Center,
								// 					align_items: AlignItems::Center,
								// 					..Default::default()
								// 				},
								// 				..Default::default()
								// 			})
								// 			.with_children(|p| {
								// 				p.spawn(TextBundle::from_section(
								// 					"X".to_string(),
								// 					TextStyle {
								// 						font: asset_server.load("fonts/FiraSans-Bold.ttf"),
								// 						font_size: 10.0,
								// 						color: Color::BLACK,
								// 					},
								// 				));
								// 			});
								// 		}
								// 	});
								// }
							},
						}
					});
				}
			}
		});
	});
}
