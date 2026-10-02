namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>The single path for skill growth, so growth curves and rank-ups live in one place.</summary>
        public void GainSkill(Cat c, string skill, double amount) => Add(c.Skills, skill, amount);
    }
}
