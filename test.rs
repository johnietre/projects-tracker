fn main() {
    let mut b = std::collections::BTreeMap::new();
    b.insert(3, 3);
    b.insert(2, 2);
    b.insert(10, 20);
    b.insert(1, 1);
    b.insert(5, 4);
    b.iter().for_each(|(k, v)| println!("{k}|{v}"));
}
