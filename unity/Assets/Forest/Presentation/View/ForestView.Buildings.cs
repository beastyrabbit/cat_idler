using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Buildings, their doorways, construction progress and the busy-workshop marker.</summary>
    public sealed partial class ForestView
    {
        private void ReconcileBuildings(Village v, int level)
        {
            foreach (var b in v.Buildings.Where(b => b.Position.Level == level))
            {
                var go = Entity("building:" + b.Id, b.Kind, At(b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f), Mathf.Min(b.Width, b.Depth) * .92f);
                Int2? entrance = b.HasEntrance ? b.Entrance : (Int2?)null;
                if (entrance.HasValue)
                {
                    var p = entrance.Value;
                    var outward = p.X < b.Position.X ? Vector3.left : p.X >= b.Position.X + b.Width ? Vector3.right : p.Z < b.Position.Z ? Vector3.back : Vector3.forward;
                    go.transform.rotation = Quaternion.LookRotation(outward);
                    Entity("entrance:" + b.Id, "road", At(p) - outward * .45f + Vector3.up * .015f, .52f);
                }
                if (!b.Completed) go.transform.localScale = new Vector3(go.transform.localScale.x, Mathf.Max(.15f, (float)b.Progress / Mathf.Max(1, (float)b.RequiredWork)) * go.transform.localScale.x, go.transform.localScale.z);
                else if (b.WorkerId != "" && !b.Paused && b.BlockedReason == "")
                    Entity("activity:" + b.Id, "cargo_tools", go.transform.position + new Vector3(.3f, .7f, .1f), .3f);
            }
        }
    }
}
