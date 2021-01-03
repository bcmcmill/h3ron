use std::collections::HashMap;
use std::iter::Iterator;
use std::os::raw::c_int;

use geo_types::Polygon;

use h3ron_h3_sys::{GeoCoord, Geofence, GeoPolygon, H3Index};

use crate::geo::linestring_to_geocoords;
use crate::index::Index;

#[macro_use]
mod util;
mod geo;
pub mod collections;
pub mod experimental;
pub mod error;
pub mod index;

pub const H3_MIN_RESOLUTION: u8 = 0_u8;
pub const H3_MAX_RESOLUTION: u8 = 15_u8;

pub enum AreaUnits {
    M2,
    Km2,
}

pub fn hex_area_at_resolution(resolution: i32, units: AreaUnits) -> f64 {
    match units {
        AreaUnits::M2 => unsafe { h3ron_h3_sys::hexAreaM2(resolution) },
        AreaUnits::Km2 => unsafe { h3ron_h3_sys::hexAreaKm2(resolution) },
    }
}

unsafe fn to_geofence(ring: &mut Vec<GeoCoord>) -> Geofence {
    Geofence {
        numVerts: ring.len() as c_int,
        verts: ring.as_mut_ptr(),
    }
}


pub fn max_polyfill_size(poly: &Polygon<f64>, h3_resolution: u8) -> usize {
    unsafe {
        let mut exterior: Vec<GeoCoord> = linestring_to_geocoords(&poly.exterior());
        let mut interiors: Vec<Vec<GeoCoord>> = poly.interiors().iter()
            .map(|ls| linestring_to_geocoords(ls))
            .collect();

        let mut holes: Vec<Geofence> = interiors
            .iter_mut()
            .map(|ring| to_geofence(ring))
            .collect();

        let gp = GeoPolygon {
            geofence: to_geofence(&mut exterior),
            numHoles: holes.len() as c_int,
            holes: holes.as_mut_ptr(),
        };

        h3ron_h3_sys::maxPolyfillSize(&gp, h3_resolution as c_int) as usize
    }
}

pub fn polyfill(poly: &Polygon<f64>, h3_resolution: u8) -> Vec<H3Index> {
    let mut h3_indexes = unsafe {
        let mut exterior: Vec<GeoCoord> = linestring_to_geocoords(&poly.exterior());
        let mut interiors: Vec<Vec<GeoCoord>> = poly.interiors().iter()
            .map(|ls| linestring_to_geocoords(ls))
            .collect();

        let mut holes: Vec<Geofence> = interiors
            .iter_mut()
            .map(|ring| to_geofence(ring))
            .collect();

        let gp = GeoPolygon {
            geofence: to_geofence(&mut exterior),
            numHoles: holes.len() as c_int,
            holes: holes.as_mut_ptr(),
        };

        let num_hexagons = h3ron_h3_sys::maxPolyfillSize(&gp, h3_resolution as c_int);

        // pre-allocate for the expected number of hexagons
        let mut h3_indexes: Vec<H3Index> = vec![0; num_hexagons as usize];

        h3ron_h3_sys::polyfill(&gp, h3_resolution as c_int, h3_indexes.as_mut_ptr());
        h3_indexes
    };
    remove_zero_indexes_from_vec!(h3_indexes);
    h3_indexes
}


///
/// the input vec must be deduplicated
pub fn compact(h3_indexes: &[H3Index]) -> Vec<H3Index> {
    let mut h3_indexes_out: Vec<H3Index> = vec![0; h3_indexes.len()];
    unsafe {
        h3ron_h3_sys::compact(h3_indexes.as_ptr(), h3_indexes_out.as_mut_ptr(), h3_indexes.len() as c_int);
    }
    remove_zero_indexes_from_vec!(h3_indexes_out);
    h3_indexes_out
}

/// compact h3indexes of mixed resolutions
pub fn compact_mixed(h3_indexes: &[H3Index]) -> Vec<H3Index> {
    let mut h3_indexes_by_res = HashMap::new();
    for h3_index in h3_indexes {
        h3_indexes_by_res.entry(Index::from(*h3_index).resolution())
            .or_insert_with(Vec::new)
            .push(*h3_index);
    }
    let mut out_h3indexes = vec![];
    for (_, res_indexes) in h3_indexes_by_res.drain() {
        let mut compacted = compact(&res_indexes);
        out_h3indexes.append(&mut compacted);
    }
    out_h3indexes
}


pub fn max_k_ring_size(k: u32) -> usize {
    unsafe { h3ron_h3_sys::maxKringSize(k as c_int) as usize }
}