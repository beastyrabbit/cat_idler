using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Resource pills (GUIDE §5.3), top right: food, water, logs and stone from the last physical count with a
    /// freshness mark (✓ within the last game hour, the age in sun when older), "≈ total" while nothing has
    /// been counted yet, then research with its rate and blessings. A value change flashes the pill and floats
    /// "+N". Clicking a pill opens Stores, Research or Shrine. Mounted in slot-pills.
    /// </summary>
    public class ResourcePills : VisualElement, IHudPart
    {
        public const string RowName = "hud-resources";
        public static readonly string[] Counted = { "food", "water", "logs", "stone" };
        public const double FreshGameHours = 1;
        public const double RateWindowSeconds = 60;

        private readonly List<Pill> pills = new List<Pill>();
        private readonly Queue<(double time, double points)> researchSamples = new Queue<(double, double)>();
        private string villageId = "";

        public ResourcePills(UiContext ctx)
        {
            Context = ctx;
            name = RowName;
            AddToClassList("res");
            pickingMode = PickingMode.Ignore;
            foreach (var resource in Counted) pills.Add(new Pill(this, resource, resource, "Stores"));
            pills.Add(new Pill(this, "research", "refined", "Research"));
            pills.Add(new Pill(this, "blessings", "blessings", "Shrine"));
            Refresh();
        }

        public UiContext Context { get; }

        public void Refresh()
        {
            var world = Context.World;
            var village = Context.Village;
            if (world == null || village == null)
            {
                AddToClassList("gone");
                return;
            }
            RemoveFromClassList("gone");
            bool reset = village.Id != villageId;
            if (reset) { villageId = village.Id; researchSamples.Clear(); }
            var storage = village.Stockpiles.Where(p => p.Kind == "storage").ToList();
            double countedAt = storage.Count == 0 ? -1 : storage.Max(p => p.CountedAt);
            foreach (var pill in pills)
            {
                switch (pill.Resource)
                {
                    case "research":
                        pill.Show(village.ResearchPoints, Whole(village.ResearchPoints), ResearchUnit(world, village), Mark.Unit, reset);
                        break;
                    case "blessings":
                        pill.Show(village.Blessings, Whole(village.Blessings), "bless", Mark.Unit, reset);
                        break;
                    default:
                        if (countedAt < 0)
                        {
                            // Before any count the Leader's coarse estimate stands in, never the exact stock (W2-02).
                            double total = world.EstimatedTotal(village, pill.Resource, out _);
                            pill.Show(total, "≈" + Whole(total), "", Mark.Estimate, reset);
                        }
                        else
                        {
                            double reported = world.ReportedTotal(village, pill.Resource);
                            bool fresh = GameHoursSince(world, countedAt) < FreshGameHours;
                            pill.Show(reported, Whole(reported), fresh ? "" : AgeText(world, countedAt), fresh ? Mark.Fresh : Mark.Stale, reset);
                        }
                        break;
                }
            }
        }

        /// <summary>"res" plus the research gained per minute over the last minute of play.</summary>
        private string ResearchUnit(World world, Village village)
        {
            double now = world.TimeSeconds;
            researchSamples.Enqueue((now, village.ResearchPoints));
            while (researchSamples.Count > 2 && now - researchSamples.Peek().time > RateWindowSeconds) researchSamples.Dequeue();
            double gained = 0, previous = double.NaN, first = researchSamples.Peek().time;
            foreach (var sample in researchSamples)
            {
                if (!double.IsNaN(previous) && sample.points > previous) gained += sample.points - previous;
                previous = sample.points;
            }
            double span = now - first;
            if (span < 5 || gained <= 0) return "res";
            double perMinute = gained / span * 60;
            return "res · +" + perMinute.ToString(perMinute >= 10 ? "0" : "0.0", CultureInfo.InvariantCulture) + "/min";
        }

        public static string Whole(double value) => Math.Floor(value + 1e-9).ToString("0", CultureInfo.InvariantCulture);

        public static double GameHoursSince(World world, double time) => Math.Max(0, world.TimeSeconds - time) * world.CurrentPacing().DayScale / 3600;

        /// <summary>A game-time age: "¼ h" under an hour, whole hours above (GUIDE §7 numbers).</summary>
        public static string AgeText(World world, double time)
        {
            double hours = GameHoursSince(world, time);
            if (hours < 1) return "¼ h";
            if (hours < 48) return Math.Floor(hours).ToString("0", CultureInfo.InvariantCulture) + " h";
            return Math.Floor(hours / 24).ToString("0", CultureInfo.InvariantCulture) + " d";
        }

        private enum Mark { Fresh, Stale, Estimate, Unit }

        private sealed class Pill
        {
            private readonly ResourcePills owner;
            private readonly Button root;
            private readonly Label count, unit, delta;
            private readonly VisualElement check;
            private double shown = double.NaN;
            private IVisualElementScheduledItem settle;

            public Pill(ResourcePills owner, string resource, string icon, string section)
            {
                this.owner = owner;
                Resource = resource;
                root = new Button(() => owner.Context.Hud.OpenPanel(section)) { name = "resource-" + resource, text = "" };
                root.AddToClassList("pill");
                root.AddToClassList("glass");
                root.AddToClassList("pill-" + resource);
                root.Add(HudIcons.Goods(icon, "pill-icon"));
                count = new Label("") { pickingMode = PickingMode.Ignore };
                count.AddToClassList("pill-count");
                root.Add(count);
                check = HudIcons.Glyph("check", "pill-check", "ok");
                root.Add(check);
                unit = new Label("") { pickingMode = PickingMode.Ignore };
                unit.AddToClassList("pill-unit");
                root.Add(unit);
                delta = new Label("") { pickingMode = PickingMode.Ignore };
                delta.AddToClassList("pill-delta");
                root.Add(delta);
                owner.Add(root);
            }

            public string Resource { get; }

            public void Show(double value, string text, string unitText, Mark mark, bool reset)
            {
                if (count.text != text) count.text = text;
                if (unit.text != unitText) unit.text = unitText;
                unit.EnableInClassList("gone", unitText.Length == 0);
                check.EnableInClassList("gone", mark != Mark.Fresh);
                root.EnableInClassList("stale", mark == Mark.Stale);
                root.EnableInClassList("estimate", mark == Mark.Estimate);
                root.EnableInClassList("scalar", mark == Mark.Unit);
                double whole = Math.Floor(value + 1e-9);
                if (!reset && !double.IsNaN(shown) && whole != shown) Changed(whole - shown);
                shown = whole;
            }

            /// <summary>Flash the pill and float the difference, unless the player asked for reduced motion.</summary>
            private void Changed(double difference)
            {
                if (GameSettings.ReducedMotion) return;
                delta.text = (difference > 0 ? "+" : "−") + Math.Abs(difference).ToString("0", CultureInfo.InvariantCulture);
                delta.EnableInClassList("down", difference < 0);
                delta.RemoveFromClassList("fade");
                delta.AddToClassList("show");
                root.AddToClassList("flash");
                settle?.Pause();
                settle = root.schedule.Execute(() =>
                {
                    root.RemoveFromClassList("flash");
                    delta.RemoveFromClassList("show");
                    delta.AddToClassList("fade");
                }).StartingIn(60);
            }
        }
    }
}
