// Copyright (c) 2018 10X Genomics, Inc. All rights reserved.

// This file contains an equivalence relation struct.  There is a literature on
// this and multiple rust implementations of sophisticated algorithms, see:
//
// 1. https://en.wikipedia.org/wiki/Disjoint-set_data_structure
// 2. https://crates.io/crates/union-find
// 3. https://crates.io/crates/disjoint-sets
// 4. https://crates.io/crates/disjoint-set [seems defunct]
// 5. https://crates.io/crates/fera-unionfind
//
// The code here is an optimized and rustified version of the code in the 10X
// supernova codebase, which was adopted from the BroadCRD codebase.  The code here
// uses a very naive algorithm that should not be competitive with the sophisticated
// algorithms, but for unknown reasons, it is.  There are some comparisons to the
// disjoint-sets crate at the end of this file.  The implementations in other
// crates were not tested.

use std::mem::swap;

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// EQUIVALENCE RELATION
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Computational performance of EquivRel:
// - storage = 3N bytes, where N is the set size; storage is flat
// - initialization time = O(N)
// - time to make n joins = O( n * log(N) )
// - time to find all orbit reps = O(N)
// - time to find an orbit = O(size of orbit)
// - time to find the size of an orbit = O(1)
// - time to find the class id of an element = O(1).

pub struct EquivRel {
    x: Vec<i32>, // next element in orbit
    y: Vec<i32>, // orbit class id
    z: Vec<i32>, // orbit size
}

impl EquivRel {
    pub fn new(n: i32) -> EquivRel {
        let mut xx: Vec<i32> = Vec::with_capacity(n as usize);
        let mut yy: Vec<i32> = Vec::with_capacity(n as usize);
        let mut zz: Vec<i32> = Vec::with_capacity(n as usize);
        for i in 0..n {
            xx.push(i);
            yy.push(i);
            zz.push(1);
        }
        EquivRel {
            x: xx,
            y: yy,
            z: zz,
        }
    }

    pub fn join(&mut self, a: i32, b: i32) {
        let mut ax = a;
        let mut bx = b;
        if self.y[ax as usize] != self.y[bx as usize] {
            // Always move the smaller orbit.  This is critical as otherwise
            // complexity of join would be O( n * N ) and not O( n * log(N) ).

            if self.orbit_size(ax) < self.orbit_size(bx) {
                swap(&mut ax, &mut bx);
            }

            // Now do the move.

            let new_size = self.orbit_size(ax) + self.orbit_size(bx);
            self.x.swap(ax as usize, bx as usize);
            let mut n = self.x[ax as usize];
            loop {
                if self.y[n as usize] == self.y[ax as usize] {
                    break;
                }
                self.y[n as usize] = self.y[ax as usize];
                n = self.x[n as usize];
            }

            // Update orbit size.

            self.z[self.y[bx as usize] as usize] = new_size;
        }
    }

    pub fn orbit_reps(&self, reps: &mut Vec<i32>) {
        reps.clear();
        for i in 0..self.x.len() {
            if i == self.y[i as usize] as usize {
                reps.push(i as i32);
            }
        }
    }

    pub fn norbits(&self) -> usize {
        let mut n = 0;
        for i in 0..self.x.len() {
            if i == self.y[i as usize] as usize {
                n += 1;
            }
        }
        n
    }

    pub fn orbit_size(&self, a: i32) -> i32 {
        self.z[self.y[a as usize] as usize]
    }

    // orbit: compute the orbit o of an element.  The simplest thing is for o
    // to be a Vec<i32>, but often it is convenient to instead have it be a
    // Vec<usize>.

    pub fn orbit<T: From<i32>>(&self, a: i32, o: &mut Vec<T>) {
        o.clear();
        // o.reserve( self.orbit_size(a) as usize ); // weirdly slower
        o.push(T::from(a));
        let mut b = a;
        loop {
            b = self.x[b as usize];
            if b == a {
                break;
            }
            o.push(T::from(b));
        }
    }

    pub fn class_id(&self, a: i32) -> i32 {
        self.y[a as usize]
    }
}

// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
// PERFORMANCE COMPARISONS
// ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓

// Comparison to the disjoint-sets crate.  Note that with disjoint sets, it is not
// clear how to find just one orbit, or the size of one orbit.  Two comparisons
// were carried out.  The first comparison follows.  Briefly, it shows that
// disjoint-sets is faster for this use case, but uses more memory, and in short,
// the conclusion is a "toss-up".

/*

    // Setup.

    const N : usize = 10_000_000;
    const L : usize = 20_000_000;
    let mut x : Vec<i64> = vec![ 0; L ];
    make_random_vec( &mut x, L );
    for i in 0..L { x[i] = x[i].abs() % (N as i64); }
    let peak = peak_mem_usage_bytes();
    let t = Instant::now( );

    // EquivRel version (use this or the following)
    // there are 1618950 orbits
    // 2.6 seconds, delta peak mem = 137 Mb

    let mut e = EquivRel::new(N as i32);
    for j in 0..L/2 { e.join( x[j] as i32, x[j+L/2] as i32 ); }
    let mut reps = Vec::<i32>::new();
    e.orbit_reps( &mut reps );
    println!( "there are {} orbits", reps.len() );
    let mut o = Vec::<i32>::new();
    for i in 0..reps.len() { e.orbit( reps[i] as i32, &mut o ); }

    // UnionFind version (use this or the previous);
    // there are 1618950 orbits
    // 1.5 seconds, delta peak mem = 258 Mb
    // disjoint-sets = "0.4.2"
    // extern crate disjoint_sets;

    use disjoint_sets::UnionFind;
    let mut uf = UnionFind::<u32>::new(N as usize);
    for j in 0..L/2 { uf.union( x[j] as u32, x[j+L/2] as u32 ); }
    let reps = uf.to_vec();
    let mut repsx = Vec::<(u32,u32)>::new();
    for i in 0..reps.len() { repsx.push( (reps[i],i as u32) ); }
    repsx.sort();
    let mut o = Vec::<u32>::new();
    let mut orbits = Vec::<Vec<u32>>::new();
    for i in 0..repsx.len() {
        if i > 0 && repsx[i].0 != repsx[i-1].0 {
            orbits.push( o.clone() );
            o.clear();
        }
        o.push( repsx[i].1 );
    }
    orbits.push( o.clone() );
    println!( "there are {} orbits", orbits.len() );

    // Summarize.

    let delta_peak = peak_mem_usage_bytes() - peak;
    println!(
        "{} seconds used, delta peak mem = {} bytes", elapsed(&t), delta_peak );

*/

// The second comparison involves a change to the code in hyper.rs.  Here is the
// relevant chunk of code, using EquivRel:

/*

    // Find nodes in the transformed graph.  They are orbits of edge ends under
    // the natural equivalence relation.

    let mut eq : EquivRel = EquivRel::new( 2 * edges.len() as i32 );
    for i in 0..adj.len() {
        let left = adj[i].0;
        let right = adj[i].1;
        eq.join( 2*left + 1, 2*right );
    }
    let mut reps = Vec::<i32>::new();
    eq.orbit_reps( &mut reps );

    // Now actually create the transformed graph.

    g_out.clear();
    g_out.reserve_exact_nodes( reps.len() );
    g_out.reserve_exact_edges( edges.len() );
    for i in 0..reps.len() { g_out.add_node(i as u32); }
    for e in 0..edges.len() {
        let v = bin_position( &reps, &eq.class_id((2*e) as i32) );
        let w = bin_position( &reps, &eq.class_id((2*e+1) as i32) );
        g_out.add_edge( NodeIndex::<u32>::new(v as usize),
            NodeIndex::<u32>::new(w as usize), edges[e].2.clone() );
    }
}

*/

// and here is the relevant chunk of code using disjoint-sets:

/*

    use disjoint_sets::UnionFind;

    // Find nodes in the transformed graph.  They are orbits of edge ends under
    // the natural equivalence relation.

    let mut eq = UnionFind::<u32>::new( 2 * edges.len() );
    for i in 0..adj.len() {
        let left = adj[i].0 as u32;
        let right = adj[i].1 as u32;
        eq.union( 2*left + 1, 2*right );
    }
    let mut reps = eq.to_vec(); // list of orbit representatives
    reps.sort();

    // Now actually create the transformed graph.

    g_out.clear();
    g_out.reserve_exact_nodes( reps.len() );
    g_out.reserve_exact_edges( edges.len() );
    for i in 0..reps.len() { g_out.add_node(i as u32); }
    for e in 0..edges.len() {
        let v = bin_position( &reps, &eq.find( (2*e) as u32) );
        let w = bin_position( &reps, &eq.find( (2*e+1) as u32) );
        g_out.add_edge( NodeIndex::<u32>::new(v as usize),
            NodeIndex::<u32>::new(w as usize), edges[e].2.clone() );

*/

// Performance comparison: the test consisted of running the assemblies for
// 14 VDJ samples.  The actual test is not described here, but the results are
// as follows:
//
// version         server seconds   peak mem GB
// EquivRel        1366.10           7.65
// disjoint-sets   1570.20          13.45
//
// Of course one ought to be able to define a reproducible test that exhibits this
// performance difference.
