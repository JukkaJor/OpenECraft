//! JT (ISO 14306) exchange of triangle meshes, after the JT 8.1 file layout: file
//! header, a zlib compressed logical scene graph (partition, group and tri-strip set
//! shape nodes with material attributes and name properties) and one shape LOD
//! segment per shape with lossless (unquantized) vertex data.
//!
//! The reader understands this subset; files with B-rep (JT B-Rep / XT) only,
//! quantized or topologically compressed LOD data are refused with a message.
//! Units are millimetres, z up.

mod read;
mod write;

pub use read::parse_jt;
pub use write::{write_jt, Body, Group};

/// A JT object type or segment identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Guid(u32, u16, u16, [u8; 8]);

const JT_SUFFIX: [u8; 8] = [0x9b, 0x6b, 0x00, 0x80, 0xc7, 0xbb, 0x59, 0x97];
const END_OF_ELEMENTS: Guid = Guid(0xffff_ffff, 0xffff, 0xffff, [0xff; 8]);
const PARTITION_NODE: Guid = Guid(0x10dd_103e, 0x2ac8, 0x11d1, JT_SUFFIX);
const GROUP_NODE: Guid = Guid(0x10dd_101b, 0x2ac8, 0x11d1, JT_SUFFIX);
const TRI_STRIP_SET_SHAPE_NODE: Guid = Guid(0x10dd_1077, 0x2ac8, 0x11d1, JT_SUFFIX);
const MATERIAL_ATTRIBUTE: Guid = Guid(0x10dd_1030, 0x2ac8, 0x11d1, JT_SUFFIX);
const STRING_PROPERTY_ATOM: Guid = Guid(0x10dd_106e, 0x2ac8, 0x11d1, JT_SUFFIX);
const LATE_LOADED_PROPERTY_ATOM: Guid = Guid(0xe0b0_5be5, 0xfbbd, 0x11d1, [0xa3, 0xa7, 0x00, 0xaa, 0x00, 0xd1, 0x09, 0x54]);
const TRI_STRIP_SET_SHAPE_LOD: Guid = Guid(0x10dd_10ab, 0x2ac8, 0x11d1, JT_SUFFIX);

/// Object base types of the element header.
const BASE_GROUP: u8 = 2;
const BASE_SHAPE: u8 = 3;
const BASE_ATTRIBUTE: u8 = 4;
const BASE_SHAPE_LOD: u8 = 5;
const BASE_PROPERTY: u8 = 6;

/// Segment types (TOC entry attributes, bits 24 – 31).
const SEGMENT_LSG: u32 = 1;
const SEGMENT_SHAPE_LOD0: u32 = 7;

const VERSION: &str = "Version 8.1 JT";
/// Length of the file header: version text, byte order, empty field, TOC offset, LSG segment ID.
const HEADER_LEN: usize = 80 + 1 + 4 + 4 + 16;
/// Length of a segment header: segment ID, type, length.
const SEGMENT_HEADER_LEN: usize = 16 + 4 + 4;
const ZLIB: u8 = 2;
