use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs::File;
use std::io::Read;
use std::cmp::{Eq, Ord, Ordering};
use wavefront_obj::obj;
use Vertex;

use util::get_assets_folder;

impl Vertex {
    fn new(v: &obj::Vertex, tex_coord: &obj::TVertex, normal: &obj::Normal) -> Self {
        Vertex {
            pos: [v.x as f32, v.y as f32, v.z as f32],
            uv: [tex_coord.u as f32, tex_coord.v as f32],
            normal: [normal.x as f32, normal.y as f32, normal.z as f32],
        }
    }
}

pub fn load_obj(obj_name: &str) -> (Vec<Vertex>, Vec<u16>) {
    use wavefront_obj::obj::Primitive;
    let mut obj_string = String::new();
    let mut obj_file_name = get_assets_folder().unwrap().to_path_buf();
    obj_file_name.push(&format!("mesh/{}.obj", obj_name));
    let mut obj_file = File::open(obj_file_name).expect(&format!("Could not open {}.obj", obj_name));
    obj_file.read_to_string(&mut obj_string).expect(&format!("Could not read {}.obj", obj_name));
    drop(obj_file);

    let obj = obj::parse(obj_string).expect(&format!("Could not parse {}.obj", obj_name));
    let object = obj.objects.get(0).expect("No objects");

    let mut verts = Vec::new();
    let mut uvs = Vec::new();
    let mut norms = Vec::new();

    for s in object.geometry.iter().flat_map(|g| g.shapes.iter()) {
        match s.primitive {
            Primitive::Triangle((i0, Some(t0), Some(n0)),
                                (i1, Some(t1), Some(n1)),
                                (i2, Some(t2), Some(n2))) => {
                verts.push(object.vertices[i0 as usize]);
                verts.push(object.vertices[i1 as usize]);
                verts.push(object.vertices[i2 as usize]);

                uvs.push(object.tex_vertices[t0 as usize]);
                uvs.push(object.tex_vertices[t1 as usize]);
                uvs.push(object.tex_vertices[t2 as usize]);

                norms.push(object.normals[n1 as usize]);
                norms.push(object.normals[n0 as usize]);
                norms.push(object.normals[n2 as usize]);
            }
            _ => unimplemented!(),
        }
    }

    build_unified_buffers(&verts[..], &uvs[..], &norms[..])
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
struct PackedObjVertex {
    pos: obj::Vertex,
    uv: obj::TVertex,
    norm: obj::Normal,
}

// @HACK: floats are not strictly comparable, we are just using this so that the
//        BTreeMap works. Note that the code I'm copying this from
//        (https://github.com/opengl-tutorials/ogl/blob/master/common/vboindexer.cpp)
//        also has this problem.
impl Eq for PackedObjVertex {}
impl Ord for PackedObjVertex {
    fn cmp(&self, other: &Self) -> Ordering {
        // Arbitrary equality for BTreeMap
        let pself = self as *const _;
        let pother = other as *const _;
        pself.cmp(&pother)
    }
}

impl PackedObjVertex {
    fn new(p: obj::Vertex, t: obj::TVertex, n: obj::Normal) -> Self {
        PackedObjVertex {
            pos: p,
            uv: t,
            norm: n,
        }
    }
}

fn build_unified_buffers(vertices: &[obj::Vertex],
                         tex_coords: &[obj::TVertex],
                         normals: &[obj::Normal])
                         -> (Vec<Vertex>, Vec<u16>) {
    let mut out_verts = Vec::new();
    let mut out_inds = Vec::new();
    let mut vert_to_out = BTreeMap::new();

    for packed in vertices.iter()
        .zip(tex_coords.iter())
        .zip(normals.iter())
        .map(|((v, t), n)| PackedObjVertex::new(*v, *t, *n)) {
        match vert_to_out.entry(packed.clone()) {
            Entry::Occupied(e) => out_inds.push(*e.get()),
            Entry::Vacant(e) => {
                out_verts.push(Vertex::new(&packed.pos, &packed.uv, &packed.norm));
                let new_index = (out_verts.len() - 1) as u16;
                out_inds.push(new_index);
                e.insert(new_index);
            }
        }
    }

    (out_verts, out_inds)
}
