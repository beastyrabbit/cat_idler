namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// The recipe the simulation uses for seconds, inputs and outputs. Every simulation
        /// lookup goes through here so a pacing profile can override recipes in one place.
        /// </summary>
        public Recipe RecipeOf(string id) => Catalog.Recipe(id);
    }
}
