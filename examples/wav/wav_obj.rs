

use wgx::error::*;
use wgx::normal_from_triangle;

use std::str::{FromStr};


fn parse_vec<'a>(line:&mut impl Iterator<Item=&'a str>) -> Res<[f32;4]> {

    let vec:Vec<f32> = line.map(f32::from_str).collect::<Result<_, _>>().convert()?;

    if vec.len() < 2 {
        Err(format!("bad vec length {}", vec.len()))
    }
    else {
        Ok([
            vec[0],
            vec[1],
            if let Some(v) = vec.get(2) { *v } else { 1.0 },
            if let Some(v) = vec.get(3) { *v } else { 1.0 },
        ])
    }
}


fn parse_face<'a>(line:&mut impl Iterator<Item=&'a str>) -> Res<Vec<(usize, Option<usize>, Option<usize>)>> {

    let face:Vec<(usize, Option<usize>, Option<usize>)> = line.map(|part| {

        let part:Vec<usize> = part.split('/').map(usize::from_str).collect::<Result<_, _>>().convert()?;

        if part.is_empty() {
            Err("bad face".to_string())
        }
        else {
            Ok((
                part[0] - 1,
                part.get(1).map(|v| v - 1),
                part.get(2).map(|v| v - 1),
            ))
        }
    }).collect::<Result<_, _>>().convert()?;


    let len = face.len();

    if !(3..=4).contains(&len) {
        Err(format!("bad face length: {}", len))
    }
    else {
        Ok(face)
    }
}


pub fn parse(raw:&str) -> Res<Vec<[[[f32;3];3];3]>> {

    let mut vertices:Vec<[f32;4]> = Vec::new();
    let mut vertex_tex_coords:Vec<[f32;3]> = Vec::new();
    let mut normals:Vec<[f32;3]> = Vec::new();

    let mut faces:Vec<Vec<(usize, Option<usize>, Option<usize>)>> = Vec::new();

    for line in raw.split('\n') {

        let mut line = line.trim().split(' ').filter(|v| v.trim() != "");

        match line.next() {
            Some("v") => { vertices.push(parse_vec(&mut line)?); }
            Some("vt") => { vertex_tex_coords.push(parse_vec(&mut line)?[0..3].try_into().unwrap()); }
            Some("vn") => { normals.push(parse_vec(&mut line)?[0..3].try_into().unwrap()); }
            Some("f") => { faces.push(parse_face(&mut line)?); }
            _ => {}
        }
    }

    let mut triangles = Vec::new();

    // let mut once = false;

    for face in faces {

        let mut calc_normals = false;

        let mut trgs = Vec::with_capacity(4);

        for (v, t, n) in face {

            if n.is_none() {
                calc_normals = true;
            }

            let [x, y, z, w] = vertices[v];

            trgs.push([
                [x/w, y/w, z/w],
                if let Some(i) = t {
                    let [x, y, w] = vertex_tex_coords[i];
                    [x/w, y/w, 0.0]
                } else { [0.0, 0.0, 0.0] },
                if let Some(i) = n { normals[i] } else { [0.0, 0.0, 0.0] },
            ]);
        }

        if calc_normals {

            let normal = normal_from_triangle(trgs[0][0], trgs[1][0], trgs[2][0]).into();

            for trg in trgs.iter_mut() { trg[2] = normal; }
        }

        triangles.push([trgs[0], trgs[1], trgs[2]]);

        if trgs.len() == 4 {
            triangles.push([trgs[0], trgs[2], trgs[3]]);
        }
    }

    Ok(triangles)
}
