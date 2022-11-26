use num_traits::float::Float;
use std::fmt::Display;

static DIM: usize = 2;
static NULL: usize = 0;
//static DEBUG: usize = 4;
static DEBUG: usize = 0; // dlogs get optimized away at 0

#[cfg(test)]
mod tests;

type LinkedListNodeIndex = usize;
type VerticesIndex = usize;

#[derive(Clone, Copy, Debug)]
struct LinkedListNode<T: Float + Display> {
    /// vertex index in flat one-d array of 64bit float coords
    vertices_index: VerticesIndex,
    /// vertex x coordinate
    x: T,
    /// vertex y coordinate
    y: T,
    /// previous vertex node in a polygon ring
    prev_linked_list_node_index: LinkedListNodeIndex,
    /// next vertex node in a polygon ring
    next_linked_list_node_index: LinkedListNodeIndex,
    /// z-order curve value
    z: i32,
    /// previous node in z-order
    prevz_idx: LinkedListNodeIndex,
    /// next node in z-order
    nextz_idx: LinkedListNodeIndex,
    /// indicates whether this is a steiner point
    is_steiner_point: bool,
    /// index within LinkedLists vector that holds all nodes
    idx: LinkedListNodeIndex,
}

impl<T: Float + Display> LinkedListNode<T> {
    fn new(i: VerticesIndex, x: T, y: T, idx: LinkedListNodeIndex) -> LinkedListNode<T> {
        LinkedListNode {
            vertices_index: i,
            x,
            y,
            prev_linked_list_node_index: NULL,
            next_linked_list_node_index: NULL,
            z: 0,
            nextz_idx: NULL,
            prevz_idx: NULL,
            is_steiner_point: false,
            idx,
        }
    }

    // check if two points are equal
    fn xy_eq(&self, other: LinkedListNode<T>) -> bool {
        self.x == other.x && self.y == other.y
    }
}

pub struct LinkedLists<T: Float + Display> {
    nodes: Vec<LinkedListNode<T>>,
    invsize: T,
    minx: T,
    miny: T,
    maxx: T,
    maxy: T,
    usehash: bool,
}

macro_rules! dlog {
	($loglevel:expr, $($s:expr),*) => (
		if DEBUG>=$loglevel { print!("{}:",$loglevel); println!($($s),+); }
	)
}

macro_rules! node {
    ($ll:expr,$idx:expr) => {
        $ll.nodes[$idx]
    };
}

// Note: none of the following macros work for Left-Hand-Side of assignment.
macro_rules! next {
    ($ll:expr,$idx:expr) => {
        $ll.nodes[$ll.nodes[$idx].next_linked_list_node_index]
    };
}
macro_rules! nextref {
    ($ll:expr,$idx:expr) => {
        &$ll.nodes[$ll.nodes[$idx].next_linked_list_node_index]
    };
}
macro_rules! prev {
    ($ll:expr,$idx:expr) => {
        $ll.nodes[$ll.nodes[$idx].prev_linked_list_node_index]
    };
}
macro_rules! prevref {
    ($ll:expr,$idx:expr) => {
        &$ll.nodes[$ll.nodes[$idx].prev_linked_list_node_index]
    };
}
macro_rules! prevz {
    ($ll:expr,$idx:expr) => {
        &$ll.nodes[$ll.nodes[$idx].prevz_idx]
    };
}

impl<T: Float + Display> LinkedLists<T> {
    fn iter(&self, r: std::ops::Range<LinkedListNodeIndex>) -> NodeIterator<T> {
        return NodeIterator::new(self, r.start, r.end);
    }
    fn iter_pairs(&self, r: std::ops::Range<LinkedListNodeIndex>) -> NodePairIterator<T> {
        return NodePairIterator::new(self, r.start, r.end);
    }
    fn insert_node(
        &mut self,
        i: VerticesIndex,
        x: T,
        y: T,
        last: Option<LinkedListNodeIndex>,
    ) -> LinkedListNodeIndex {
        let mut p = LinkedListNode::new(i, x, y, self.nodes.len());
        match last {
            None => {
                p.next_linked_list_node_index = p.idx;
                p.prev_linked_list_node_index = p.idx;
            }
            Some(last) => {
                p.next_linked_list_node_index = self.nodes[last].next_linked_list_node_index;
                p.prev_linked_list_node_index = last;
                let lastnextidx = self.nodes[last].next_linked_list_node_index;
                self.nodes[lastnextidx].prev_linked_list_node_index = p.idx;
                self.nodes[last].next_linked_list_node_index = p.idx;
            }
        }
        let result = p.idx;
        self.nodes.push(p);
        result
    }
    fn remove_node(&mut self, p_idx: LinkedListNodeIndex) {
        let pi = self.nodes[p_idx].prev_linked_list_node_index;
        let ni = self.nodes[p_idx].next_linked_list_node_index;
        let pz = self.nodes[p_idx].prevz_idx;
        let nz = self.nodes[p_idx].nextz_idx;
        self.nodes[pi].next_linked_list_node_index = ni;
        self.nodes[ni].prev_linked_list_node_index = pi;
        self.nodes[pz].nextz_idx = nz;
        self.nodes[nz].prevz_idx = pz;
    }
    fn new(size_hint: usize) -> LinkedLists<T> {
        let mut ll = LinkedLists {
            nodes: Vec::with_capacity(size_hint),
            invsize: T::zero(),
            minx: T::max_value(),
            miny: T::max_value(),
            maxx: T::min_value(),
            maxy: T::min_value(),
            usehash: true,
        };
        // ll.nodes[0] is the NULL node. For example usage, see remove_node()
        ll.nodes.push(LinkedListNode {
            vertices_index: 0,
            x: T::zero(),
            y: T::zero(),
            prev_linked_list_node_index: 0,
            next_linked_list_node_index: 0,
            z: 0,
            nextz_idx: 0,
            prevz_idx: 0,
            is_steiner_point: false,
            idx: 0,
        });
        ll
    }
}

struct NodeIterator<'a, T: Float + Display> {
    cur: LinkedListNodeIndex,
    end: LinkedListNodeIndex,
    ll: &'a LinkedLists<T>,
    pending_result: Option<&'a LinkedListNode<T>>,
}

impl<'a, T: Float + Display> NodeIterator<'a, T> {
    fn new(
        ll: &LinkedLists<T>,
        start: LinkedListNodeIndex,
        end: LinkedListNodeIndex,
    ) -> NodeIterator<T> {
        NodeIterator {
            pending_result: Some(&ll.nodes[start]),
            cur: start,
            end,
            ll,
        }
    }
}

impl<'a, T: Float + Display> Iterator for NodeIterator<'a, T> {
    type Item = &'a LinkedListNode<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.cur = self.ll.nodes[self.cur].next_linked_list_node_index;
        let cur_result = self.pending_result;
        if self.cur == self.end {
            // only one branch, saves time
            self.pending_result = None;
        } else {
            self.pending_result = Some(&self.ll.nodes[self.cur]);
        }
        cur_result
    }
}

struct NodePairIterator<'a, T: Float + Display> {
    cur: LinkedListNodeIndex,
    end: LinkedListNodeIndex,
    ll: &'a LinkedLists<T>,
    pending_result: Option<(&'a LinkedListNode<T>, &'a LinkedListNode<T>)>,
}

impl<'a, T: Float + Display> NodePairIterator<'a, T> {
    fn new(
        ll: &LinkedLists<T>,
        start: LinkedListNodeIndex,
        end: LinkedListNodeIndex,
    ) -> NodePairIterator<T> {
        NodePairIterator {
            pending_result: Some((&ll.nodes[start], nextref!(ll, start))),
            cur: start,
            end,
            ll,
        }
    }
}

impl<'a, T: Float + Display> Iterator for NodePairIterator<'a, T> {
    type Item = (&'a LinkedListNode<T>, &'a LinkedListNode<T>);
    fn next(&mut self) -> Option<Self::Item> {
        self.cur = node!(self.ll, self.cur).next_linked_list_node_index;
        let cur_result = self.pending_result;
        if self.cur == self.end {
            // only one branch, saves time
            self.pending_result = None;
        } else {
            self.pending_result = Some((&self.ll.nodes[self.cur], nextref!(self.ll, self.cur)))
        }
        cur_result
    }
}

fn compare_x<T: Float + Display>(
    a: &LinkedListNode<T>,
    b: &LinkedListNode<T>,
) -> std::cmp::Ordering {
    a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
}

// link every hole into the outer loop, producing a single-ring polygon
// without holes
fn eliminate_holes<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    vertices: &[T],
    hole_indices: &[VerticesIndex],
    inouter_node: LinkedListNodeIndex,
) -> LinkedListNodeIndex {
    let mut outer_node = inouter_node;
    let mut queue: Vec<LinkedListNode<T>> = Vec::new();
    for i in 0..hole_indices.len() {
        let vertices_hole_start_index = hole_indices[i] * DIM;
        let vertices_hole_end_index = if i < (hole_indices.len() - 1) {
            hole_indices[i + 1] * DIM
        } else {
            vertices.len()
        };
        let (list, leftmost_idx) = linked_list_add_contour(
            ll,
            vertices,
            vertices_hole_start_index,
            vertices_hole_end_index,
            false,
        );
        if list == ll.nodes[list].next_linked_list_node_index {
            ll.nodes[list].is_steiner_point = true;
        }
        queue.push(node!(ll, leftmost_idx));
    }

    queue.sort_by(compare_x);

    // process holes from left to right
    for node in queue {
        eliminate_hole(ll, node.idx, outer_node);
        let nextidx = next!(ll, outer_node).idx;
        outer_node = filter_points(ll, outer_node, Some(nextidx));
    }
    outer_node
} // elim holes

// minx, miny and invsize are later used to transform coords
// into integers for z-order calculation
fn calc_invsize<T: Float + Display>(minx: T, miny: T, maxx: T, maxy: T) -> T {
    let invsize = T::max(maxx - minx, maxy - miny);
    match invsize.is_zero() {
        true => T::zero(),
        false => num_traits::cast::<f64, T>(32767.0).unwrap() / invsize,
    }
}

// main ear slicing loop which triangulates a polygon (given as a linked
// list)
fn earcut_linked_hashed<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    mut ear_idx: LinkedListNodeIndex,
    triangle_indices: &mut FinalTriangleIndices,
    pass: usize,
) {
    // interlink polygon nodes in z-order
    if pass == 0 {
        index_curve(ll, ear_idx);
    }
    // iterate through ears, slicing them one by one
    let mut stop_idx = ear_idx;
    let mut prev_idx = 0;
    let mut next_idx = node!(ll, ear_idx).next_linked_list_node_index;
    while stop_idx != next_idx {
        prev_idx = node!(ll, ear_idx).prev_linked_list_node_index;
        next_idx = node!(ll, ear_idx).next_linked_list_node_index;
        let node_index_triangle = NodeIndexTriangle(prev_idx, ear_idx, next_idx);
        if node_index_triangle.node_triangle(ll).is_ear_hashed(ll) {
            triangle_indices.push(VerticesIndexTriangle(
                node!(ll, prev_idx).vertices_index,
                node!(ll, ear_idx).vertices_index,
                node!(ll, next_idx).vertices_index,
            ));
            ll.remove_node(ear_idx);
            // skipping the next vertex leads to less sliver triangles
            ear_idx = node!(ll, next_idx).next_linked_list_node_index;
            stop_idx = ear_idx;
        } else {
            ear_idx = next_idx;
        }
    }

    if prev_idx == next_idx {
        return;
    };
    // if we looped through the whole remaining polygon and can't
    // find any more ears
    if pass == 0 {
        let tmp = filter_points(ll, next_idx, None);
        earcut_linked_hashed(ll, tmp, triangle_indices, 1);
    } else if pass == 1 {
        ear_idx = cure_local_intersections(ll, next_idx, triangle_indices);
        earcut_linked_hashed(ll, ear_idx, triangle_indices, 2);
    } else if pass == 2 {
        split_earcut(ll, next_idx, triangle_indices);
    }
}

// main ear slicing loop which triangulates a polygon (given as a linked
// list)
fn earcut_linked_unhashed<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    mut ear_idx: LinkedListNodeIndex,
    triangles: &mut FinalTriangleIndices,
    pass: usize,
) {
    // iterate through ears, slicing them one by one
    let mut stop_idx = ear_idx;
    let mut prev_idx = 0;
    let mut next_idx = node!(ll, ear_idx).next_linked_list_node_index;
    while stop_idx != next_idx {
        prev_idx = node!(ll, ear_idx).prev_linked_list_node_index;
        next_idx = node!(ll, ear_idx).next_linked_list_node_index;
        if NodeIndexTriangle(prev_idx, ear_idx, next_idx).is_ear(ll) {
            triangles.push(VerticesIndexTriangle(
                node!(ll, prev_idx).vertices_index,
                node!(ll, ear_idx).vertices_index,
                node!(ll, next_idx).vertices_index,
            ));
            ll.remove_node(ear_idx);
            // skipping the next vertex leads to less sliver triangles
            ear_idx = node!(ll, next_idx).next_linked_list_node_index;
            stop_idx = ear_idx;
        } else {
            ear_idx = next_idx;
        }
    }

    if prev_idx == next_idx {
        return;
    };
    // if we looped through the whole remaining polygon and can't
    // find any more ears
    if pass == 0 {
        let tmp = filter_points(ll, next_idx, None);
        earcut_linked_unhashed(ll, tmp, triangles, 1);
    } else if pass == 1 {
        ear_idx = cure_local_intersections(ll, next_idx, triangles);
        earcut_linked_unhashed(ll, ear_idx, triangles, 2);
    } else if pass == 2 {
        split_earcut(ll, next_idx, triangles);
    }
}

// interlink polygon nodes in z-order
fn index_curve<T: Float + Display>(ll: &mut LinkedLists<T>, start: LinkedListNodeIndex) {
    let invsize = ll.invsize;
    let mut p = start;
    loop {
        if node!(ll, p).z == 0 {
            ll.nodes[p].z = zorder(node!(ll, p).x, node!(ll, p).y, invsize);
        }
        ll.nodes[p].prevz_idx = node!(ll, p).prev_linked_list_node_index;
        ll.nodes[p].nextz_idx = node!(ll, p).next_linked_list_node_index;
        p = node!(ll, p).next_linked_list_node_index;
        if p == start {
            break;
        }
    }

    let pzi = prevz!(ll, start).idx;
    ll.nodes[pzi].nextz_idx = NULL;
    ll.nodes[start].prevz_idx = NULL;
    sort_linked(ll, start);
}

// Simon Tatham's linked list merge sort algorithm
// http://www.chiark.greenend.org.uk/~sgtatham/algorithms/listsort.html
fn sort_linked<T: Float + Display>(ll: &mut LinkedLists<T>, mut list: LinkedListNodeIndex) {
    let mut p;
    let mut q;
    let mut e;
    let mut nummerges;
    let mut psize;
    let mut qsize;
    let mut insize = 1;
    let mut tail;

    loop {
        p = list;
        list = NULL;
        tail = NULL;
        nummerges = 0;

        while p != NULL {
            nummerges += 1;
            q = p;
            psize = 0;
            while q != NULL && psize < insize {
                psize += 1;
                q = ll.nodes[q].nextz_idx;
            }
            qsize = insize;

            while psize > 0 || (qsize > 0 && q != NULL) {
                if psize > 0 && (qsize == 0 || q == NULL || ll.nodes[p].z <= ll.nodes[q].z) {
                    e = p;
                    p = ll.nodes[p].nextz_idx;
                    psize -= 1;
                } else {
                    e = q;
                    q = ll.nodes[q].nextz_idx;
                    qsize -= 1;
                }

                if tail != NULL {
                    ll.nodes[tail].nextz_idx = e;
                } else {
                    list = e;
                }

                ll.nodes[e].prevz_idx = tail;
                tail = e;
            }

            p = q;
        }

        ll.nodes[tail].nextz_idx = NULL;
        insize *= 2;
        if nummerges <= 1 {
            break;
        }
    }
}

#[derive(Clone, Copy)]
struct NodeIndexTriangle(
    LinkedListNodeIndex,
    LinkedListNodeIndex,
    LinkedListNodeIndex,
);

impl NodeIndexTriangle {
    fn prev_node<T: Float + Display>(self, ll: &LinkedLists<T>) -> LinkedListNode<T> {
        ll.nodes[self.0]
    }

    fn ear_node<T: Float + Display>(self, ll: &LinkedLists<T>) -> LinkedListNode<T> {
        ll.nodes[self.1]
    }

    fn next_node<T: Float + Display>(self, ll: &LinkedLists<T>) -> LinkedListNode<T> {
        ll.nodes[self.2]
    }

    fn node_triangle<T: Float + Display>(self, ll: &LinkedLists<T>) -> NodeTriangle<T> {
        NodeTriangle(self.prev_node(ll), self.ear_node(ll), self.next_node(ll))
    }

    fn area<T: Float + Display>(self, ll: &LinkedLists<T>) -> T {
        self.node_triangle(ll).area()
    }

    // check whether a polygon node forms a valid ear with adjacent nodes
    fn is_ear<T: Float + Display>(self, ll: &LinkedLists<T>) -> bool {
        let zero = T::zero();
        match self.area(ll) >= zero {
            true => false, // reflex, cant be ear
            false => !ll
                .iter(self.next_node(ll).next_linked_list_node_index..self.prev_node(ll).idx)
                .any(|p| {
                    point_in_triangle(
                        self.prev_node(ll),
                        self.ear_node(ll),
                        self.next_node(ll),
                        *p,
                    ) && (NodeTriangle(*prevref!(ll, p.idx), *p, *nextref!(ll, p.idx)).area()
                        >= zero)
                }),
        }
    }
}

#[derive(Clone, Copy)]
struct NodeTriangle<T: Float + Display>(LinkedListNode<T>, LinkedListNode<T>, LinkedListNode<T>);

impl<T: Float + Display> NodeTriangle<T> {
    fn from_ear_node(ear_node: LinkedListNode<T>, ll: &mut LinkedLists<T>) -> Self {
        NodeTriangle(
            ll.nodes[ear_node.prev_linked_list_node_index],
            ear_node,
            ll.nodes[ear_node.next_linked_list_node_index],
        )
    }

    fn area(&self) -> T {
        let p = self.0;
        let q = self.1;
        let r = self.2;
        // signed area of a parallelogram
        (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y)
    }

    #[inline(always)]
    fn is_ear_hashed(&self, ll: &mut LinkedLists<T>) -> bool {
        let zero = T::zero();

        if self.area() >= zero {
            return false;
        };
        let NodeTriangle(prev, ear, next) = self;

        let bbox_maxx = T::max(prev.x, T::max(ear.x, next.x));
        let bbox_maxy = T::max(prev.y, T::max(ear.y, next.y));
        let bbox_minx = T::min(prev.x, T::min(ear.x, next.x));
        let bbox_miny = T::min(prev.y, T::min(ear.y, next.y));
        // z-order range for the current triangle bbox;
        let min_z = zorder(bbox_minx, bbox_miny, ll.invsize);
        let max_z = zorder(bbox_maxx, bbox_maxy, ll.invsize);

        let mut p = ear.prevz_idx;
        let mut n = ear.nextz_idx;
        while (p != NULL) && (node!(ll, p).z >= min_z) && (n != NULL) && (node!(ll, n).z <= max_z) {
            if earcheck(
                prev,
                ear,
                next,
                prevref!(ll, p),
                &ll.nodes[p],
                nextref!(ll, p),
            ) {
                return false;
            }
            p = node!(ll, p).prevz_idx;

            if earcheck(
                prev,
                ear,
                next,
                prevref!(ll, n),
                &ll.nodes[n],
                nextref!(ll, n),
            ) {
                return false;
            }
            n = node!(ll, n).nextz_idx;
        }

        ll.nodes[NULL].z = min_z - 1;
        while node!(ll, p).z >= min_z {
            if earcheck(
                prev,
                ear,
                next,
                prevref!(ll, p),
                &ll.nodes[p],
                nextref!(ll, p),
            ) {
                return false;
            }
            p = node!(ll, p).prevz_idx;
        }

        ll.nodes[NULL].z = max_z + 1;
        while node!(ll, n).z <= max_z {
            if earcheck(
                prev,
                ear,
                next,
                prevref!(ll, n),
                &ll.nodes[n],
                nextref!(ll, n),
            ) {
                return false;
            }
            n = node!(ll, n).nextz_idx;
        }

        true
    }
}

// helper for is_ear_hashed. needs manual inline (rust 2018)
#[inline(always)]
fn earcheck<T: Float + Display>(
    a: &LinkedListNode<T>,
    b: &LinkedListNode<T>,
    c: &LinkedListNode<T>,
    prev: &LinkedListNode<T>,
    p: &LinkedListNode<T>,
    next: &LinkedListNode<T>,
) -> bool {
    let zero = T::zero();

    (p.idx != a.idx)
        && (p.idx != c.idx)
        && point_in_triangle(*a, *b, *c, *p)
        && NodeTriangle(*prev, *p, *next).area() >= zero
}

fn filter_points<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    start: LinkedListNodeIndex,
    end: Option<LinkedListNodeIndex>,
) -> LinkedListNodeIndex {
    dlog!(
        4,
        "fn filter_points, eliminate colinear or duplicate points"
    );
    let mut end = end.unwrap_or(start);
    if end >= ll.nodes.len() || start >= ll.nodes.len() {
        return NULL;
    }

    let mut p = start;
    let mut again;

    // this loop "wastes" calculations by going over the same points multiple
    // times. however, altering the location of the 'end' node can disrupt
    // the algorithm of other code that calls the filter_points function.
    loop {
        again = false;
        if !node!(ll, p).is_steiner_point
            && (ll.nodes[p].xy_eq(ll.nodes[ll.nodes[p].next_linked_list_node_index])
                || NodeTriangle::from_ear_node(ll.nodes[p], ll)
                    .area()
                    .is_zero())
        {
            ll.remove_node(p);
            end = ll.nodes[p].prev_linked_list_node_index;
            p = end;
            if p == ll.nodes[p].next_linked_list_node_index {
                break end;
            }
            again = true;
        } else {
            debug_assert!(
                p != ll.nodes[p].next_linked_list_node_index,
                "the next node cannot be the current node"
            );
            p = ll.nodes[p].next_linked_list_node_index;
        }
        if !again && p == end {
            break end;
        }
    }
}

// create a circular doubly linked list from polygon points in the
// specified winding order
fn linked_list<T: Float + Display>(
    vertices: &[T],
    start: usize,
    end: usize,
    clockwise: bool,
) -> (LinkedLists<T>, LinkedListNodeIndex) {
    let mut ll: LinkedLists<T> = LinkedLists::new(vertices.len() / DIM);
    if vertices.len() < 80 {
        ll.usehash = false
    };
    let (last_idx, _) = linked_list_add_contour(&mut ll, vertices, start, end, clockwise);
    (ll, last_idx)
}

// add new nodes to an existing linked list.
fn linked_list_add_contour<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    vertices: &[T],
    start: VerticesIndex,
    end: VerticesIndex,
    clockwise: bool,
) -> (LinkedListNodeIndex, LinkedListNodeIndex) {
    assert!(start <= vertices.len() && end <= vertices.len() && !vertices.is_empty());
    // Previous code:
    //
    // if start > vertices.len() || end > vertices.len() || vertices.is_empty() {
    //     return (None, None);
    // }
    let mut lastidx = None;
    let mut leftmost_idx = None;
    let mut contour_minx = T::max_value();

    if clockwise == (signed_area(vertices, start, end) > T::zero()) {
        for i in (start..end).step_by(DIM) {
            lastidx = Some(ll.insert_node(i / DIM, vertices[i], vertices[i + 1], lastidx));
            if contour_minx > vertices[i] {
                contour_minx = vertices[i];
                leftmost_idx = lastidx
            };
            if ll.usehash {
                ll.miny = T::min(vertices[i + 1], ll.miny);
                ll.maxx = T::max(vertices[i], ll.maxx);
                ll.maxy = T::max(vertices[i + 1], ll.maxy);
            }
        }
    } else {
        for i in (start..=(end - DIM)).rev().step_by(DIM) {
            lastidx = Some(ll.insert_node(i / DIM, vertices[i], vertices[i + 1], lastidx));
            if contour_minx > vertices[i] {
                contour_minx = vertices[i];
                leftmost_idx = lastidx
            };
            if ll.usehash {
                ll.miny = T::min(vertices[i + 1], ll.miny);
                ll.maxx = T::max(vertices[i], ll.maxx);
                ll.maxy = T::max(vertices[i + 1], ll.maxy);
            }
        }
    }

    ll.minx = T::min(contour_minx, ll.minx);

    if ll.nodes[lastidx.unwrap()].xy_eq(*nextref!(ll, lastidx.unwrap())) {
        ll.remove_node(lastidx.unwrap());
        lastidx = Some(ll.nodes[lastidx.unwrap()].next_linked_list_node_index);
    }
    (lastidx.unwrap(), leftmost_idx.unwrap())
}

// z-order of a point given coords and inverse of the longer side of
// data bbox
#[inline(always)]
fn zorder<T: Float + Display>(xf: T, yf: T, invsize: T) -> i32 {
    // coords are transformed into non-negative 15-bit integer range
    // stored in two 32bit ints, which are combined into a single 64 bit int.
    let x: i64 = num_traits::cast::<T, i64>(xf * invsize).unwrap();
    let y: i64 = num_traits::cast::<T, i64>(yf * invsize).unwrap();
    let mut xy: i64 = x << 32 | y;

    // todo ... big endian?
    xy = (xy | (xy << 8)) & 0x00FF00FF00FF00FF;
    xy = (xy | (xy << 4)) & 0x0F0F0F0F0F0F0F0F;
    xy = (xy | (xy << 2)) & 0x3333333333333333;
    xy = (xy | (xy << 1)) & 0x5555555555555555;

    ((xy >> 32) | (xy << 1)) as i32
}

// check if a point lies within a convex triangle
fn point_in_triangle<T: Float + Display>(
    a: LinkedListNode<T>,
    b: LinkedListNode<T>,
    c: LinkedListNode<T>,
    p: LinkedListNode<T>,
) -> bool {
    let zero = T::zero();

    ((c.x - p.x) * (a.y - p.y) - (a.x - p.x) * (c.y - p.y) >= zero)
        && ((a.x - p.x) * (b.y - p.y) - (b.x - p.x) * (a.y - p.y) >= zero)
        && ((b.x - p.x) * (c.y - p.y) - (c.x - p.x) * (b.y - p.y) >= zero)
}

struct VerticesIndexTriangle(usize, usize, usize);

#[derive(Default, Debug)]
struct FinalTriangleIndices(Vec<usize>);

impl FinalTriangleIndices {
    fn push(&mut self, vertices_index_triangle: VerticesIndexTriangle) {
        self.0.push(vertices_index_triangle.0);
        self.0.push(vertices_index_triangle.1);
        self.0.push(vertices_index_triangle.2);
    }
}

pub fn earcut<T: Float + Display>(
    vertices: &[T],
    hole_indices: &[usize],
    dims: usize,
) -> Vec<usize> {
    if vertices.is_empty() {
        return vec![];
    }

    let outer_len = match hole_indices.len() {
        0 => vertices.len(),
        _ => hole_indices[0] * DIM,
    };

    let (mut ll, outer_node) = linked_list(vertices, 0, outer_len, true);
    let mut triangles = FinalTriangleIndices(Vec::with_capacity(vertices.len() / DIM));
    if ll.nodes.len() == 1 || DIM != dims {
        return triangles.0;
    }

    let outer_node = eliminate_holes(&mut ll, vertices, hole_indices, outer_node);

    if ll.usehash {
        ll.invsize = calc_invsize(ll.minx, ll.miny, ll.maxx, ll.maxy);

        // translate all points so min is 0,0. prevents subtraction inside
        // zorder. also note invsize does not depend on translation in space
        // if one were translating in a space with an even spaced grid of points.
        // floating point space is not evenly spaced, but it is close enough for
        // this hash algorithm
        let (mx, my) = (ll.minx, ll.miny);
        ll.nodes.iter_mut().for_each(|n| n.x = n.x - mx);
        ll.nodes.iter_mut().for_each(|n| n.y = n.y - my);
        earcut_linked_hashed(&mut ll, outer_node, &mut triangles, 0);
    } else {
        earcut_linked_unhashed(&mut ll, outer_node, &mut triangles, 0);
    }

    triangles.0
}

/* go through all polygon nodes and cure small local self-intersections
what is a small local self-intersection? well, lets say you have four points
a,b,c,d. now imagine you have three line segments, a-b, b-c, and c-d. now
imagine two of those segments overlap each other. thats an intersection. so
this will remove one of those nodes so there is no more overlap.

but theres another important aspect of this function. it will dump triangles
into the 'triangles' variable, thus this is part of the triangulation
algorithm itself.*/
fn cure_local_intersections<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    instart: LinkedListNodeIndex,
    triangles: &mut FinalTriangleIndices,
) -> LinkedListNodeIndex {
    let mut p = instart;
    let mut start = instart;

    //        2--3  4--5 << 2-3 + 4-5 pseudointersects
    //           x  x
    //  0  1  2  3  4  5  6  7
    //  a  p  pn b
    //              eq     a      b
    //              psi    a p pn b
    //              li  pa a p pn b bn
    //              tp     a p    b
    //              rn       p pn
    //              nst    a      p pn b
    //                            st

    //
    //                            a p  pn b

    loop {
        let a = node!(ll, p).prev_linked_list_node_index;
        let b = next!(ll, p).next_linked_list_node_index;

        if !ll.nodes[a].xy_eq(ll.nodes[b])
            && pseudo_intersects(
                ll.nodes[a],
                ll.nodes[p],
                *nextref!(ll, p),
                ll.nodes[b],
            )
			// prev next a, prev next b
            && locally_inside(ll, &ll.nodes[a], &ll.nodes[b])
            && locally_inside(ll, &ll.nodes[b], &ll.nodes[a])
        {
            triangles.push(VerticesIndexTriangle(
                ll.nodes[a].vertices_index,
                ll.nodes[p].vertices_index,
                ll.nodes[b].vertices_index,
            ));

            // remove two nodes involved
            ll.remove_node(p);
            let nidx = ll.nodes[p].next_linked_list_node_index;
            ll.remove_node(nidx);

            start = ll.nodes[b].idx;
            p = start;
        }
        p = ll.nodes[p].next_linked_list_node_index;
        if p == start {
            break;
        }
    }

    p
}

// try splitting polygon into two and triangulate them independently
fn split_earcut<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    start_idx: LinkedListNodeIndex,
    triangles: &mut FinalTriangleIndices,
) {
    // look for a valid diagonal that divides the polygon into two
    let mut a = start_idx;
    loop {
        let mut b = next!(ll, a).next_linked_list_node_index;
        while b != ll.nodes[a].prev_linked_list_node_index {
            if ll.nodes[a].vertices_index != ll.nodes[b].vertices_index
                && is_valid_diagonal(ll, &ll.nodes[a], &ll.nodes[b])
            {
                // split the polygon in two by the diagonal
                let mut c = split_bridge_polygon(ll, a, b);

                // filter colinear points around the cuts
                let an = ll.nodes[a].next_linked_list_node_index;
                let cn = ll.nodes[c].next_linked_list_node_index;
                a = filter_points(ll, a, Some(an));
                c = filter_points(ll, c, Some(cn));

                // run earcut on each half
                earcut_linked_hashed(ll, a, triangles, 0);
                earcut_linked_hashed(ll, c, triangles, 0);
                return;
            }
            b = ll.nodes[b].next_linked_list_node_index;
        }
        a = ll.nodes[a].next_linked_list_node_index;
        if a == start_idx {
            break;
        }
    }
}

// find a bridge between vertices that connects hole with an outer ring
// and and link it
fn eliminate_hole<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    hole_idx: LinkedListNodeIndex,
    outer_node_idx: LinkedListNodeIndex,
) {
    let test_idx = find_hole_bridge(ll, hole_idx, outer_node_idx);
    let b = split_bridge_polygon(ll, test_idx, hole_idx);
    let ni = node!(ll, b).next_linked_list_node_index;
    filter_points(ll, b, Some(ni));
}

// David Eberly's algorithm for finding a bridge between hole and outer polygon
fn find_hole_bridge<T: Float + Display>(
    ll: &LinkedLists<T>,
    hole: LinkedListNodeIndex,
    outer_node: LinkedListNodeIndex,
) -> LinkedListNodeIndex {
    let mut p = outer_node;
    let hx = node!(ll, hole).x;
    let hy = node!(ll, hole).y;
    let mut qx = T::neg_infinity();
    let mut m: Option<LinkedListNodeIndex> = None;

    // find a segment intersected by a ray from the hole's leftmost
    // point to the left; segment's endpoint with lesser x will be
    // potential connection point
    let calcx = |p: &LinkedListNode<T>| {
        p.x + (hy - p.y) * (next!(ll, p.idx).x - p.x) / (next!(ll, p.idx).y - p.y)
    };
    for (p, n) in ll
        .iter_pairs(p..outer_node)
        .filter(|(p, n)| hy <= p.y && hy >= n.y)
        .filter(|(p, n)| n.y != p.y)
        .filter(|(p, _)| calcx(p) <= hx)
    {
        if qx < calcx(p) {
            qx = calcx(p);
            if qx == hx && hy == p.y {
                return p.idx;
            } else if qx == hx && hy == n.y {
                return p.next_linked_list_node_index;
            }
            m = if p.x < n.x { Some(p.idx) } else { Some(n.idx) };
        }
    }

    let Some(m) = m else { return NULL };

    // hole touches outer segment; pick lower endpoint
    if hx == qx {
        return prev!(ll, m).idx;
    }

    // look for points inside the triangle of hole point, segment
    // intersection and endpoint; if there are no points found, we have
    // a valid connection; otherwise choose the point of the minimum
    // angle with the ray as connection point

    let mp = LinkedListNode::new(0, node!(ll, m).x, node!(ll, m).y, 0);
    p = next!(ll, m).idx;
    let x1 = if hy < mp.y { hx } else { qx };
    let x2 = if hy < mp.y { qx } else { hx };
    let n1 = LinkedListNode::new(0, x1, hy, 0);
    let n2 = LinkedListNode::new(0, x2, hy, 0);
    let two = num_traits::cast::<f64, T>(2.).unwrap();

    let calctan = |p: &LinkedListNode<T>| (hy - p.y).abs() / (hx - p.x); // tangential
    ll.iter(p..m)
        .filter(|p| hx > p.x && p.x >= mp.x)
        .filter(|p| point_in_triangle(n1, mp, n2, **p))
        .fold((m, T::max_value() / two), |(m, tan_min), p| {
            if ((calctan(p) < tan_min) || (calctan(p) == tan_min && p.x > ll.nodes[m].x))
                && locally_inside(ll, p, &ll.nodes[hole])
            {
                (p.idx, calctan(p))
            } else {
                (m, tan_min)
            }
        })
        .0
}

// check if a diagonal between two polygon nodes is valid (lies in
// polygon interior)
fn is_valid_diagonal<T: Float + Display>(
    ll: &LinkedLists<T>,
    a: &LinkedListNode<T>,
    b: &LinkedListNode<T>,
) -> bool {
    next!(ll, a.idx).vertices_index != b.vertices_index
        && prev!(ll, a.idx).vertices_index != b.vertices_index
        && !intersects_polygon(ll, *a, *b)
        && locally_inside(ll, a, b)
        && locally_inside(ll, b, a)
        && middle_inside(ll, a, b)
}

/* check if two segments cross over each other. note this is different
from pure intersction. only two segments crossing over at some interior
point is considered intersection.

line segment p1-q1 vs line segment p2-q2.

note that if they are collinear, or if the end points touch, or if
one touches the other at one point, it is not considered an intersection.

please note that the other algorithms in this earcut code depend on this
interpretation of the concept of intersection - if this is modified
so that endpoint touching qualifies as intersection, then it will have
a problem with certain inputs.

bsed on https://www.geeksforgeeks.org/check-if-two-given-line-segments-intersect/

this has been modified from the version in earcut.js to remove the
detection for endpoint detection.

    a1=area(p1,q1,p2);a2=area(p1,q1,q2);a3=area(p2,q2,p1);a4=area(p2,q2,q1);
    p1 q1    a1 cw   a2 cw   a3 ccw   a4  ccw  a1==a2  a3==a4  fl
    p2 q2
    p1 p2    a1 ccw  a2 ccw  a3 cw    a4  cw   a1==a2  a3==a4  fl
    q1 q2
    p1 q2    a1 ccw  a2 ccw  a3 ccw   a4  ccw  a1==a2  a3==a4  fl
    q1 p2
    p1 q2    a1 cw   a2 ccw  a3 ccw   a4  cw   a1!=a2  a3!=a4  tr
    p2 q1
*/

fn pseudo_intersects<T: Float + Display>(
    p1: LinkedListNode<T>,
    q1: LinkedListNode<T>,
    p2: LinkedListNode<T>,
    q2: LinkedListNode<T>,
) -> bool {
    if (p1.xy_eq(p2) && q1.xy_eq(q2)) || (p1.xy_eq(q2) && q1.xy_eq(p2)) {
        return true;
    }
    let zero = T::zero();

    (NodeTriangle(p1, q1, p2).area() > zero) != (NodeTriangle(p1, q1, q2).area() > zero)
        && (NodeTriangle(p2, q2, p1).area() > zero) != (NodeTriangle(p2, q2, q1).area() > zero)
}

// check if a polygon diagonal intersects any polygon segments
fn intersects_polygon<T: Float + Display>(
    ll: &LinkedLists<T>,
    a: LinkedListNode<T>,
    b: LinkedListNode<T>,
) -> bool {
    ll.iter_pairs(a.idx..a.idx).any(|(p, n)| {
        p.vertices_index != a.vertices_index
            && n.vertices_index != a.vertices_index
            && p.vertices_index != b.vertices_index
            && n.vertices_index != b.vertices_index
            && pseudo_intersects(*p, *n, a, b)
    })
}

// check if a polygon diagonal is locally inside the polygon
fn locally_inside<T: Float + Display>(
    ll: &LinkedLists<T>,
    a: &LinkedListNode<T>,
    b: &LinkedListNode<T>,
) -> bool {
    let zero = T::zero();

    match NodeTriangle(*prevref!(ll, a.idx), *a, *nextref!(ll, a.idx)).area() < zero {
        true => {
            NodeTriangle(*a, *b, *nextref!(ll, a.idx)).area() >= zero
                && NodeTriangle(*a, *prevref!(ll, a.idx), *b).area() >= zero
        }
        false => {
            NodeTriangle(*a, *b, *prevref!(ll, a.idx)).area() < zero
                || NodeTriangle(*a, *nextref!(ll, a.idx), *b).area() < zero
        }
    }
}

// check if the middle point of a polygon diagonal is inside the polygon
fn middle_inside<T: Float + Display>(
    ll: &LinkedLists<T>,
    a: &LinkedListNode<T>,
    b: &LinkedListNode<T>,
) -> bool {
    let two = num_traits::cast::<f64, T>(2.0).unwrap();

    let (mx, my) = ((a.x + b.x) / two, (a.y + b.y) / two);
    ll.iter_pairs(a.idx..a.idx)
        .filter(|(p, n)| (p.y > my) != (n.y > my))
        .filter(|(p, n)| n.y != p.y)
        .filter(|(p, n)| (mx) < ((n.x - p.x) * (my - p.y) / (n.y - p.y) + p.x))
        .fold(false, |inside, _| !inside)
}

/* link two polygon vertices with a bridge;

if the vertices belong to the same linked list, this splits the list
into two new lists, representing two new polygons.

if the vertices belong to separate linked lists, it merges them into a
single linked list.

For example imagine 6 points, labeled with numbers 0 thru 5, in a single cycle.
Now split at points 1 and 4. The 2 new polygon cycles will be like this:
0 1 4 5 0 1 ...  and  1 2 3 4 1 2 3 .... However because we are using linked
lists of nodes, there will be two new nodes, copies of points 1 and 4. So:
the new cycles will be through nodes 0 1 4 5 0 1 ... and 2 3 6 7 2 3 6 7 .

splitting algorithm:

.0...1...2...3...4...5...     6     7
5p1 0a2 1m3 2n4 3b5 4q0      .c.   .d.

an<-2     an = a.next,
bp<-3     bp = b.prev;
1.n<-4    a.next = b;
4.p<-1    b.prev = a;
6.n<-2    c.next = an;
2.p<-6    an.prev = c;
7.n<-6    d.next = c;
6.p<-7    c.prev = d;
3.n<-7    bp.next = d;
7.p<-3    d.prev = bp;

result of split:
<0...1> <2...3> <4...5>      <6....7>
5p1 0a4 6m3 2n7 1b5 4q0      7c2  3d6
      x x     x x            x x  x x    // x shows links changed

a b q p a b q p  // begin at a, go next (new cycle 1)
a p q b a p q b  // begin at a, go prev (new cycle 1)
m n d c m n d c  // begin at m, go next (new cycle 2)
m c d n m c d n  // begin at m, go prev (new cycle 2)

Now imagine that we have two cycles, and
they are 0 1 2, and 3 4 5. Split at points 1 and
4 will result in a single, long cycle,
0 1 4 5 3 7 6 2 0 1 4 5 ..., where 6 and 1 have the
same x y f64s, as do 7 and 4.

 0...1...2   3...4...5        6     7
2p1 0a2 1m0 5n4 3b5 4q3      .c.   .d.

an<-2     an = a.next,
bp<-3     bp = b.prev;
1.n<-4    a.next = b;
4.p<-1    b.prev = a;
6.n<-2    c.next = an;
2.p<-6    an.prev = c;
7.n<-6    d.next = c;
6.p<-7    c.prev = d;
3.n<-7    bp.next = d;
7.p<-3    d.prev = bp;

result of split:
 0...1...2   3...4...5        6.....7
2p1 0a4 6m0 5n7 1b5 4q3      7c2   3d6
      x x     x x            x x   x x

a b q n d c m p a b q n d c m .. // begin at a, go next
a p m c d n q b a p m c d n q .. // begin at a, go prev

Return value.

Return value is the new node, at point 7.
*/
fn split_bridge_polygon<T: Float + Display>(
    ll: &mut LinkedLists<T>,
    a: LinkedListNodeIndex,
    b: LinkedListNodeIndex,
) -> LinkedListNodeIndex {
    let cidx = ll.nodes.len();
    let didx = cidx + 1;
    let mut c = LinkedListNode::new(
        ll.nodes[a].vertices_index,
        ll.nodes[a].x,
        ll.nodes[a].y,
        cidx,
    );
    let mut d = LinkedListNode::new(
        ll.nodes[b].vertices_index,
        ll.nodes[b].x,
        ll.nodes[b].y,
        didx,
    );

    let an = ll.nodes[a].next_linked_list_node_index;
    let bp = ll.nodes[b].prev_linked_list_node_index;

    ll.nodes[a].next_linked_list_node_index = b;
    ll.nodes[b].prev_linked_list_node_index = a;

    c.next_linked_list_node_index = an;
    ll.nodes[an].prev_linked_list_node_index = cidx;

    d.next_linked_list_node_index = cidx;
    c.prev_linked_list_node_index = didx;

    ll.nodes[bp].next_linked_list_node_index = didx;
    d.prev_linked_list_node_index = bp;

    ll.nodes.push(c);
    ll.nodes.push(d);
    didx
}

// return a percentage difference between the polygon area and its
// triangulation area; used to verify correctness of triangulation
pub fn deviation<T: Float + Display>(
    vertices: &[T],
    hole_indices: &[usize],
    dims: usize,
    triangles: &[usize],
) -> T {
    if DIM != dims {
        return T::nan();
    }
    let mut indices = hole_indices.to_vec();
    indices.push(vertices.len() / DIM);
    let (ix, iy) = (indices.iter(), indices.iter().skip(1));
    let body_area = signed_area(vertices, 0, indices[0] * DIM).abs();
    let polygon_area = ix.zip(iy).fold(body_area, |a, (ix, iy)| {
        a - signed_area(vertices, ix * DIM, iy * DIM).abs()
    });

    let i = triangles.iter().skip(0).step_by(3).map(|x| x * DIM);
    let j = triangles.iter().skip(1).step_by(3).map(|x| x * DIM);
    let k = triangles.iter().skip(2).step_by(3).map(|x| x * DIM);
    let triangles_area = i.zip(j).zip(k).fold(T::zero(), |ta, ((a, b), c)| {
        ta + ((vertices[a] - vertices[c]) * (vertices[b + 1] - vertices[a + 1])
            - (vertices[a] - vertices[b]) * (vertices[c + 1] - vertices[a + 1]))
            .abs()
    });

    match polygon_area.is_zero() && triangles_area.is_zero() {
        true => T::zero(),
        false => ((triangles_area - polygon_area) / polygon_area).abs(),
    }
}

fn signed_area<T: Float + Display>(vertices: &[T], start: VerticesIndex, end: VerticesIndex) -> T {
    let i = (start..end).step_by(DIM);
    let j = (start..end).cycle().skip((end - DIM) - start).step_by(DIM);
    let zero = T::zero();
    i.zip(j).fold(zero, |s, (i, j)| {
        s + (vertices[j] - vertices[i]) * (vertices[i + 1] + vertices[j + 1])
    })
}

// turn a polygon in a multi-dimensional array form (e.g. as in GeoJSON)
// into a form Earcut accepts
pub fn flatten<T: Float + Display>(data: &Vec<Vec<Vec<T>>>) -> (Vec<T>, Vec<usize>, usize) {
    (
        data.iter().flatten().flatten().cloned().collect::<Vec<T>>(), // flat data
        data.iter()
            .take(data.len() - 1)
            .scan(0, |holeidx, v| {
                *holeidx += v.len();
                Some(*holeidx)
            })
            .collect::<Vec<usize>>(), // hole indexes
        data[0][0].len(),                                             // dimensions
    )
}

fn pn(a: usize) -> String {
    match a {
        0x777A91CC => String::from("NULL"),
        _ => a.to_string(),
    }
}
fn pb(a: bool) -> String {
    match a {
        true => String::from("x"),
        false => String::from(" "),
    }
}

#[allow(dead_code)]
fn dump<T: Float + Display>(ll: &LinkedLists<T>) -> String {
    let mut s = format!("LL, #nodes: {}", ll.nodes.len());
    s.push_str(&format!(
        " #used: {}\n",
        //        ll.nodes.len() as i64 - ll.freelist.len() as i64
        ll.nodes.len() as i64
    ));
    s.push_str(&format!(
        " {:>3} {:>3} {:>4} {:>4} {:>8.3} {:>8.3} {:>4} {:>4} {:>2} {:>2} {:>2} {:>4}\n",
        "vi", "i", "p", "n", "x", "y", "pz", "nz", "st", "fr", "cyl", "z"
    ));
    for n in &ll.nodes {
        s.push_str(&format!(
            " {:>3} {:>3} {:>4} {:>4} {:>8.3} {:>8.3} {:>4} {:>4} {:>2} {:>2} {:>2} {:>4}\n",
            n.idx,
            n.vertices_index,
            pn(n.prev_linked_list_node_index),
            pn(n.next_linked_list_node_index),
            n.x,
            n.y,
            pn(n.prevz_idx),
            pn(n.nextz_idx),
            pb(n.is_steiner_point),
            false,
            //            pb(ll.freelist.contains(&n.idx)),
            0, //,ll.iter(n.idx..n.idx).count(),
            n.z,
        ));
    }
    s
}

#[allow(dead_code)]
fn cycle_dump<T: Float + Display>(ll: &LinkedLists<T>, p: LinkedListNodeIndex) -> String {
    let mut s = format!("cycle from {}, ", p);
    s.push_str(&format!(" len {}, idxs:", 0)); //cycle_len(&ll, p)));
    let mut i = p;
    let end = i;
    let mut count = 0;
    loop {
        count += 1;
        s.push_str(&format!("{} ", &ll.nodes[i].idx));
        s.push_str(&format!("(i:{}), ", &ll.nodes[i].vertices_index));
        i = ll.nodes[i].next_linked_list_node_index;
        if i == end {
            break s;
        }
        if count > ll.nodes.len() {
            s.push_str(" infinite loop");
            break s;
        }
    }
}
