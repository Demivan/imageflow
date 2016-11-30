use ::internal_prelude::works_everywhere::*;
use std::sync::atomic::{AtomicU64, Ordering, ATOMIC_U64_INIT};

static NEXT_FLUENT_NODE_ID: AtomicU64 = ATOMIC_U64_INIT;


pub fn fluently() -> FluentNode{
    FluentNode::empty()
}

#[derive(Clone,Debug)]
pub struct FluentNode{
    input: Option<Box<FluentNode>>,
    canvas: Option<Box<FluentNode>>,
    data: Option<s::Node>,
    uid: u64
}

impl FluentNode{
    fn next_uid() -> u64{
        NEXT_FLUENT_NODE_ID.fetch_add(1, Ordering::SeqCst)
    }
    fn new(node: s::Node, input_node: Option<FluentNode>, canvas_node: Option<FluentNode>) -> FluentNode{
        FluentNode{
            input: input_node.and_then(|v| match v.is_empty() { true => None, false => Some(Box::new(v))}),
            canvas: canvas_node.and_then(|v| match v.is_empty() { true => None, false => Some(Box::new(v))}),
            data: Some(node),
            uid: FluentNode::next_uid()
        }
    }
    pub fn empty() -> FluentNode{
        FluentNode{
            input: None,
            canvas: None,
            data: None,
            uid: FluentNode::next_uid()
        }
    }


    pub fn is_empty(&self) -> bool{
        self.data.is_none()
    }

    pub fn to(self, v: s::Node) -> FluentNode{
        FluentNode::new(v, Some(self), None)
    }
    pub fn to_canvas(self, canvas: FluentNode, v: s::Node) -> FluentNode {
        FluentNode::new(v, Some(self), Some(canvas))
    }
    pub fn branch(&self) -> FluentNode{
        self.clone()
    }
    pub fn builder(self) -> FluentGraphBuilder{
        FluentGraphBuilder::new_with(self)
    }

    pub fn canvas_bgra32(self,  w: usize,
                         // camelCased: #[serde(rename="fromY")]
                         h: usize, color: s::Color) -> FluentNode {
        self.to(s::Node::CreateCanvas {
            w: w,
            h: h,
            format: s::PixelFormat::Bgra32,
            color: color
        })
    }



    pub fn create_canvas(self, w: usize,
                          // camelCased: #[serde(rename="fromY")]
                          h: usize,
                          format: s::PixelFormat, color: s::Color) -> FluentNode {
        self.to(s::Node::CreateCanvas {
            w: w,
            h: h,
            format: format,
            color: color
        })
    }


    pub fn decode(self, io_id: i32) -> FluentNode{
        self.to(s::Node::Decode{io_id: io_id, commands: None})
    }

    pub fn flip_vertical(self) -> FluentNode{
        self.to(s::Node::FlipV)
    }

    pub fn flip_horizontal(self) -> FluentNode{
        self.to(s::Node::FlipH)
    }

    pub fn rotate_90(self) -> FluentNode{
        self.to(s::Node::Rotate90)
    }
    pub fn rotate_180(self) -> FluentNode{
        self.to(s::Node::Rotate180)
    }
    pub fn rotate_270(self) -> FluentNode{
        self.to(s::Node::Rotate270)
    }

    pub fn transpose(self) -> FluentNode{
        self.to(s::Node::Transpose)
    }
    pub fn copy_rect_from(self, from: FluentNode, from_x: u32,
                          // camelCased: #[serde(rename="fromY")]
                          from_y: u32,
                          width: u32,
                          height: u32,
                          x: u32,
                          y: u32,) -> FluentNode {
        from.to_canvas(self, s::Node::CopyRectToCanvas {
            from_x: from_x,
            from_y: from_y,
            width: width,
            height: height,
            x: x,
            y: y
        })
    }

}
impl PartialEq for FluentNode {
    fn eq(&self, other: &FluentNode) -> bool {
        self.uid == other.uid
    }
}



pub struct FluentGraphBuilder{
    output_nodes: Vec<Box<FluentNode>>
}

impl FluentGraphBuilder{
    pub fn new() -> FluentGraphBuilder{
        FluentGraphBuilder{
            output_nodes: vec![]
        }
    }
    pub fn new_with(n: FluentNode) -> FluentGraphBuilder{
        FluentGraphBuilder{
            output_nodes: vec![Box::new(n)]
        }
    }

    pub fn with(self, n: FluentNode) -> FluentGraphBuilder{
        let mut new_vec = self.output_nodes.clone();
        new_vec.push(Box::new(n));
        FluentGraphBuilder{
            output_nodes: new_vec
        }
    }

    fn collect_unique(&self) -> Vec<&Box<FluentNode>>{
        let mut set = HashSet::new();
        let mut todo = Vec::new();
        let mut unique = Vec::new();
        for end in self.output_nodes.as_slice().iter(){
           todo.push(end);
        }

        loop{
            if todo.len() == 0{
                break;
            }
            let next = todo.pop().unwrap();
            if !set.contains(&next.uid){
                set.insert(next.uid);
                unique.push(next);
                if let Some(ref c) = next.canvas {
                    todo.push(&c);
                }
                if let Some(ref c) = next.input {
                    todo.push(&c);
                }
            }
        }

        unique
    }

    fn collect_edges(&self, for_nodes: &[&Box<FluentNode>]) -> Vec<(u64,u64,s::EdgeKind)> {
        let mut edges = vec![];
        for n in for_nodes {
            if let Some(ref parent) = n.canvas {
                edges.push((parent.uid, n.uid, s::EdgeKind::Canvas));
            }
            if let Some(ref parent) = n.input {
                edges.push((parent.uid, n.uid, s::EdgeKind::Input));
            }
        }
        edges
    }
    fn lowest_uid(for_nodes: &[&Box<FluentNode>]) -> Option<u64>{
        for_nodes.iter().map(|n| n.uid).min()
    }
    pub fn to_framewise(&self) -> s::Framewise {
        let nodes = self.collect_unique();
        let lowest_uid = FluentGraphBuilder::lowest_uid(&nodes).unwrap_or(0);
        let edges = self.collect_edges(&nodes);
        let framewise_edges = edges.into_iter().map(|(from,to,kind)| s::Edge{from: (from - lowest_uid) as i32, to: (to - lowest_uid) as i32, kind: kind }).collect::<Vec<s::Edge>>();
        let mut framewise_nodes = HashMap::new();
        for n in nodes{
            let _ = framewise_nodes.insert((n.uid - lowest_uid).to_string(), n.data.clone().unwrap());
        }
        s::Framewise::Graph(
            s::Graph{
                edges: framewise_edges,
                nodes: framewise_nodes
            }
        )
    }

//    pub fn to_graph(&self) -> ::Graph {
//        let mut uid_map = HashMap::new();
//        let from_list = self.collect_unique();
//
//        let mut g = ::Graph::with_capacity(from_list.len(), from_list.len() + 8);
//        for n in from_list.as_slice(){
//            if let Some(ref data) = n.data {
//                let ix = g.add_node(::flow::definitions::Node::from(data.clone()));
//                uid_map.insert(n.uid, ix);
//            }
//        }
//
//        for n in from_list.as_slice(){
//            if let Some(ref parent) = n.canvas{
//                g.add_edge(uid_map[&parent.uid], uid_map[&n.uid],::ffi::EdgeKind::Canvas).unwrap();
//            }
//            if let Some(ref parent) = n.input{
//                g.add_edge(uid_map[&parent.uid], uid_map[&n.uid],::ffi::EdgeKind::Input).unwrap();
//            }
//        }
//        g
//    }
}

#[test]
fn test_graph_builder(){

    //let d = fluently().decode(0).flip_horizontal().rotate_90().
    let a = fluently().to(s::Node::CreateCanvas{w: 200, h: 200, format: s::PixelFormat::Bgra32, color: s::Color::Black}).to(s::Node::FlipV);
    let b = a.branch().to(s::Node::Encode{preset: s::EncoderPreset::libjpegturbo(), io_id: 0});
    let c= a.branch().to(s::Node::Resample2D{w: 100, h: 100, down_filter: None, up_filter: None, hints: None}).to(s::Node::Encode{ preset: s::EncoderPreset::libpng32(), io_id: 1});
    b.builder().with(c).to_framewise();
}