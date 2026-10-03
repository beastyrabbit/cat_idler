using System;
using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    public partial class Job
    {
        /// <summary>
        /// Simulation time of the job's last observed progress (a move, phase change, work progress or a cargo
        /// change). -1 until the job is first ticked; saves written before the watchdog load as -1 too.
        /// </summary>
        public double LastProgressAt = -1;
    }

    /// <summary>A job source (a pile or a work tile) the job watchdog took out of rotation until <see cref="Until"/>.</summary>
    [Serializable]
    public partial class SourceExclusion
    {
        public string Key = "";
        public double Until;
    }

    public partial class Village
    {
        /// <summary>Sources a wedged job gave up on; Request and claim candidates skip them until they expire.</summary>
        public List<SourceExclusion> SourceExclusions = new List<SourceExclusion>();
    }
}
