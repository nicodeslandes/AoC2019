use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

#[derive(Debug)]
struct GraphNode {
    key: String,
    parent: Option<String>,
    children: HashSet<String>,
}

impl<'a> GraphNode {
    fn new(key: &String, parent: Option<String>) -> GraphNode {
        GraphNode {
            key: key.clone(),
            parent: parent,
            children: HashSet::new(),
        }
    }
}

#[derive(Debug)]
struct Graph {
    nodes: HashMap<String, GraphNode>,
}

impl<'a> Graph {
    fn new() -> Graph {
        Graph {
            nodes: HashMap::new(),
        }
    }

    fn add_node_link(&mut self, parent: &String, child: &String) {
        self.add_or_get_node(child, &Some(parent.clone()));
        let parent_node = self.add_or_get_node(parent, &None);

        parent_node.children.insert(child.clone());
    }

    fn add_or_get_node(&mut self, key: &String, parent: &Option<String>) -> &mut GraphNode {
        let node = self
            .nodes
            .entry(key.clone())
            .or_insert(GraphNode::new(key, parent.clone()));

        match parent {
            Some(_p) => node.parent = parent.clone(),
            _ => (),
        };

        node
    }

    fn get_node(&self, key: &String) -> Option<&GraphNode> {
        self.nodes.get(key)
    }
}

fn main() -> Result<()> {
    let file_name = env::args().nth(1).expect("Enter a file name");

    let file = File::open(file_name)?;
    let reader = BufReader::new(file);

    let mut orbits = Graph::new();

    for (_, line) in reader.lines().enumerate() {
        let orbit = parse_line(line.unwrap());
        orbits.add_node_link(&orbit[0], &orbit[1])
    }
    //println!("Values: {:?}", orbits);

    let mut orbit_counts: HashMap<String, u32> = HashMap::new();
    for key in orbits.nodes.keys() {
        if !orbit_counts.contains_key(key) {
            compute_orbit_count_for(&orbits, &mut orbit_counts, key)
        }
    }

    println!("Orbit counts: {:?}", orbit_counts);

    println!("Total count: {}", orbit_counts.values().sum::<u32>());

    let you_node_parent = &orbits.get_node(&"YOU".to_string()).unwrap().parent;
    let santa_node_parent = &orbits.get_node(&"SAN".to_string()).unwrap().parent;

    let common_parent = find_common_parent(
        &orbits,
        &orbit_counts,
        &"YOU".to_string(),
        &"SAN".to_string(),
    );

    println!("Common parent: {}", common_parent);
    Ok(())
}
fn find_common_parent<'a>(
    orbits: &'a Graph,
    orbit_counts: &HashMap<String, u32>,
    key1: &String,
    key2: &String,
) -> &'a String {
    let key1_parents: HashSet<&String> = get_node_parents(&orbits, key1);
    let key2_parents: HashSet<&String> = get_node_parents(&orbits, key2);

    println!("Key1 parents: {:?}", key1_parents);
    println!("Key2 parents: {:?}", key2_parents);

    key1_parents
        .intersection(&key2_parents)
        .max_by_key(|key| orbit_counts.get(&key.to_string()).unwrap())
        .unwrap()
}

fn get_node_parents<'a>(orbits: &'a Graph, key: &String) -> HashSet<&'a String> {
    let mut key_parents: HashSet<&'a String> = HashSet::new();

    let mut node = orbits.get_node(key);
    while node.is_some() {
        let parent = &node.unwrap().parent;
        match parent {
            Some(p) => {
                key_parents.insert(&p);
                node = orbits.get_node(&p);
            }
            None => node = None,
        }
    }

    key_parents
}

fn compute_orbit_count_for(orbits: &Graph, orbit_counts: &mut HashMap<String, u32>, key: &String) {
    let node = orbits.get_node(key).unwrap();

    // First, compute the orbit counts for the children
    for child in node.children.iter() {
        compute_orbit_count_for(orbits, orbit_counts, &child);
    }

    // Then for the node
    let count = match node.parent {
        None => 0, // No orbit for the graph root
        Some(_) => {
            1 // direct orbit
            + node.children.iter().map(|c| orbit_counts.get(c).unwrap()).sum::<u32>()
        } // indirect orbits
    };

    orbit_counts.insert(node.key.clone(), count);
}

fn parse_line<'a>(line: String) -> Vec<String> {
    let split = line.split(")").map(|s| s.to_string());
    split.collect::<Vec<_>>()
}