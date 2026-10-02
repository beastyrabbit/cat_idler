namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// The recipe the simulation uses for seconds, inputs and outputs. Every simulation
        /// lookup goes through here: classic worlds get the catalog recipe itself, demo worlds
        /// get <see cref="DemoContent"/>'s override (and the demo-only recipes).
        /// </summary>
        public Recipe RecipeOf(string id) => DemoContent.Recipe(this, id);
    }
}
