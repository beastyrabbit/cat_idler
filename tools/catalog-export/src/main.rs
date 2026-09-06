fn main() {
    let catalog = cat_sim::research_catalog::research_catalog();
    let recipes: Vec<_> = cat_sim::types::BuildingType::ALL.iter().copied()
        .filter_map(cat_sim::station_recipes::station_recipe_set)
        .flat_map(|set| set.recipes).map(|r| serde_json::json!({
            "id":r.id,"building":r.building_type,"inputs":r.input_resources,
            "outputs":r.output_resources,"founding":r.founding_available,
            "item":r.output_item.map(|i|serde_json::json!({"kind":i.kind,"material":i.material,"quality":i.quality}))
        })).collect();
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"research":catalog.nodes(),"recipes":recipes})).unwrap());
}
