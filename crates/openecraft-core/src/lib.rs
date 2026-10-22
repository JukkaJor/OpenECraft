//! Headless project model of OpenECraft, shared by the editor and the command line /
//! MCP front end: pages, symbols, devices, netlist, checks, reports, panel layout and
//! the SVG / PDF / DXF export. Contains no UI code.

pub mod climate;
pub mod config;
pub mod dxf;
pub mod forms;
pub mod geom;
pub mod harness;
pub mod history;
pub mod index;
pub mod macros;
pub mod marking;
pub mod model;
pub mod netlist;
pub mod ops;
pub mod panel;
pub mod parts;
pub mod pdf;
pub mod plc;
pub mod plugs;
pub mod potentials;
pub mod production;
pub mod render;
pub mod reports;
pub mod revision;
pub mod sample;
pub mod sizing;
pub mod strips;
pub mod structure;
pub mod svg;
pub mod symbols;
pub mod symlib;
pub mod tables;
pub mod wirenum;
