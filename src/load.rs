use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::cmp::{Eq, Ord, Ordering};
use util::{GetAssetsFolderError, get_assets_folder};
use wavefront_obj::{ParseError, obj};
use Vertex;

pub type Index = u16;

impl Vertex {
    fn new(v: &obj::Vertex, tex_coord: &obj::TVertex, normal: &obj::Normal) -> Self {
        Vertex {
            pos: [v.x as f32, v.y as f32, v.z as f32],
            uv: [tex_coord.u as f32, tex_coord.v as f32],
            normal: [normal.x as f32, normal.y as f32, normal.z as f32],
        }
    }
}

pub fn load_obj(obj_name: &str) -> Result<(Vec<Vertex>, Vec<Index>), LoadObjError> {
    use wavefront_obj::obj::Primitive;
    let mut obj_string = String::new();
    {
        let mut obj_file_name = get_assets_folder().map(|p| p.to_path_buf())?;
        obj_file_name.push(&format!("mesh/{}.obj", obj_name));
        File::open(obj_file_name).and_then(|mut f| {
            f.read_to_string(&mut obj_string)
        })?;
    }

    let obj = obj::parse(obj_string)?;
    let object = match obj.objects.get(0) {
        Some(o) => o,
        None => return Err(LoadObjError::NoMeshFound),
    };

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
            _ => {
                println!("{:?}", s);
                unimplemented!()
            }
        }
    }

    Ok(build_unified_buffers(&verts[..], &uvs[..], &norms[..]))
}

#[derive(Debug)]
pub enum LoadObjError {
    Io(io::Error),
    ObjParse(ParseError),
    AssetsFolder(GetAssetsFolderError),
    NoMeshFound,
}

impl From<io::Error> for LoadObjError {
    #[inline]
    fn from(e: io::Error) -> Self {
        LoadObjError::Io(e)
    }
}

impl From<ParseError> for LoadObjError {
    #[inline]
    fn from(e: ParseError) -> Self {
        LoadObjError::ObjParse(e)
    }
}

impl From<GetAssetsFolderError> for LoadObjError {
    #[inline]
    fn from(e: GetAssetsFolderError) -> Self {
        LoadObjError::AssetsFolder(e)
    }
}

impl fmt::Display for LoadObjError {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LoadObjError::Io(ref e) => write!(fmtr, "{}: {}", self.description(), e),
            LoadObjError::ObjParse(ref e) => write!(fmtr, "{}, {:?}", self.description(), e),
            LoadObjError::AssetsFolder(ref e) => write!(fmtr, "{}, {:?}", self.description(), e),
            LoadObjError::NoMeshFound => fmtr.pad(self.description()),
        }
    }
}

impl Error for LoadObjError {
    #[inline]
    fn description(&self) -> &str {
        match *self {
            LoadObjError::Io(_) => "An I/O error occurred",
            LoadObjError::ObjParse(_) => "Could not parse Obj file",
            LoadObjError::AssetsFolder(_) => "Could not get assets folder",
            LoadObjError::NoMeshFound => "Could not find a mesh in the obj file",
        }
    }

    #[inline]
    fn cause(&self) -> Option<&Error> {
        match *self {
            LoadObjError::Io(ref e) => Some(e),
            LoadObjError::AssetsFolder(ref e) => Some(e),
            _ => None,
        }
    }
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
        let pself = self as *const PackedObjVertex;
        let pother = other as *const PackedObjVertex;
        pself.cmp(&pother)
    }
}

impl PackedObjVertex {
    #[inline]
    fn new(p: obj::Vertex, t: obj::TVertex, n: obj::Normal) -> Self {
        PackedObjVertex {
            pos: p,
            uv: t,
            norm: n,
        }
    }
}

fn build_unified_buffers(
    vertices: &[obj::Vertex],
    tex_coords: &[obj::TVertex],
    normals: &[obj::Normal],
) -> (Vec<Vertex>, Vec<Index>) {
    let mut out_verts = Vec::new();
    let mut out_inds = Vec::new();
    let mut vert_to_out = BTreeMap::new();

    for packed in vertices
        .iter()
        .zip(tex_coords.iter())
        .zip(normals.iter())
        .map(|((v, t), n)| PackedObjVertex::new(*v, *t, *n))
    {
        match vert_to_out.entry(packed.clone()) {
            Entry::Occupied(e) => out_inds.push(*e.get()),
            Entry::Vacant(e) => {
                out_verts.push(Vertex::new(&packed.pos, &packed.uv, &packed.norm));
                let new_index = (out_verts.len() - 1) as Index;
                out_inds.push(new_index);
                e.insert(new_index);
            }
        }
    }

    (out_verts, out_inds)
}
