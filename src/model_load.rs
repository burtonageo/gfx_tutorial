use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs::File;
use std::io::Read;
use std::cmp::{Eq, Ord, Ordering};
use wavefront_obj::obj;
use Vertex;


impl Vertex {
    fn new(v: &obj::Vertex, normal: &obj::Normal) -> Self {
        Vertex {
            pos: [v.x as f32, v.y as f32, v.z as f32],
            col: [0.3, 0.3, 0.3],
            normal: [normal.x as f32, normal.y as f32, normal.z as f32],
        }
    }
}

pub fn load_obj(obj_name: &str) -> (Vec<Vertex>, Vec<u16>) {
    use wavefront_obj::obj::Primitive;
    let mut obj_string = String::new();
    let mut obj_file_name = env!("CARGO_MANIFEST_DIR").to_string();
    obj_file_name.push_str(&format!("/data/mesh/{}.obj", obj_name));
    let mut obj_file = File::open(obj_file_name).expect("Could not open suzanne.obj");
    obj_file.read_to_string(&mut obj_string).expect("Could not read suzanne.obj");
    drop(obj_file);

    let obj = obj::parse(obj_string).expect("Could not parse suzanne.obj");
    let object = obj.objects.get(0).expect("No objects");

    let mut verts = Vec::new();
    let mut norms = Vec::new();

    for s in object.geometry.iter().flat_map(|g| g.shapes.iter()) {
        match s.primitive {
            Primitive::Triangle((i0, _, Some(n0)), (i1, _, Some(n1)), (i2, _, Some(n2))) => {
                verts.push(object.vertices[i0 as usize]);
                norms.push(object.normals[n0 as usize]);
                verts.push(object.vertices[i1 as usize]);
                norms.push(object.normals[n1 as usize]);
                verts.push(object.vertices[i2 as usize]);
                norms.push(object.normals[n2 as usize]);
            }
            _ => unimplemented!(),
        }
    }

    build_unified_buffers(&verts[..], &norms[..])
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
struct PackedObjVertex {
    pos: obj::Vertex,
    norm: obj::Normal,
}

// @HACK: floats are not strictly comparable, we are just using this so that the BTreeMap works. Note that the
//        code I'm copying this from (https://github.com/opengl-tutorials/ogl/blob/master/common/vboindexer.cpp)
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
    fn new(p: obj::Vertex, n: obj::Normal) -> Self {
        PackedObjVertex { pos: p, norm: n }
    }
}

fn build_unified_buffers(vertices: &[obj::Vertex],
                         normals: &[obj::Normal])
                         -> (Vec<Vertex>, Vec<u16>) {
    let mut out_verts = Vec::new();
    let mut out_inds = Vec::new();
    let mut vert_to_out = BTreeMap::new();

    for (v, n) in vertices.iter().zip(normals.iter()) {
        let packed = PackedObjVertex::new(*v, *n);
        match vert_to_out.entry(packed.clone()) {
            Entry::Occupied(e) => out_inds.push(*e.get()),
            Entry::Vacant(e) => {
                out_verts.push(Vertex::new(&packed.pos, &packed.norm));
                let new_index = (out_verts.len() - 1) as u16;
                out_inds.push(new_index);
                e.insert(new_index);
            }
        }
    }

    (out_verts, out_inds)
}
