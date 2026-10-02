using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The tethered card body for a cat (GUIDE 5.5): facts "Hauler · adult · Den 2", the now line, need rings
    /// (hunger, thirst, rest, health; ≥ 60 ok, 35–59 sun, &lt; 35 bad), Help, Assign to… and Control, and "More…"
    /// with needs, skills, work preferences, equipment, cargo, bed and the cat's recent events.
    /// Element names: <c>card-cat</c>, <c>ring-hunger|thirst|rest|health</c>, <c>live-work-progress</c>,
    /// <c>act-help-&lt;cat&gt;</c>, <c>act-assign-to-&lt;cat&gt;</c>, <c>act-assign-&lt;cat&gt;-&lt;building&gt;</c>,
    /// <c>act-release-workplace-&lt;cat&gt;</c>, <c>act-control-&lt;cat&gt;</c>, <c>act-more-card</c>,
    /// <c>act-labor-&lt;labor&gt;</c> (Toggle), <c>act-unequip-&lt;item&gt;</c>, <c>act-prefer-&lt;cat&gt;</c>,
    /// <c>carried-item-&lt;item&gt;</c>.
    /// </summary>
    public class CatCard : CardBody
    {
        private static readonly string[] Needs = { "hunger", "thirst", "rest", "health" };
        private readonly Dictionary<string, NeedRing> rings = new Dictionary<string, NeedRing>();
        private ProgressBar work;
        private Label needLine;
        private bool assignOpen;

        public CatCard(UiContext ctx) : base(ctx, "cat") { }

        public override bool Exists => Cat != null;

        private Cat Cat => Village?.Cats.Find(c => c.Id == SubjectId);

        protected override void Describe()
        {
            var c = Cat;
            Title = c.Name;
            PortraitEntity = "cat:" + c.Id;
            var facts = new List<string> { RoleOf(Village, c), LifeStage(World, c) };
            if (c.BedId != "")
            {
                var bed = Village.Buildings.Find(b => b.Id == c.BedId);
                if (bed != null) facts.Add(CardFormat.BuildingName(Village, bed));
            }
            else if (c.Alive) facts.Add("no bed");
            if (c.PregnantUntil >= 0) facts.Add("expecting");
            Facts = string.Join(" · ", facts.Where(f => f != ""));
            if (!c.Alive)
            {
                Now = "Remembered by the colony";
                Why = ""; WhyIsProblem = false;
                return;
            }
            Now = CardFormat.Sentence(UiText.NowLine(World, Village, c));
            var job = Village.Jobs.Find(j => j.Id == c.JobId);
            string reason = c.BlockedReason != "" ? c.BlockedReason : job?.BlockedReason ?? "";
            if (reason != "") { Why = UiText.HumanReason(reason); WhyIsProblem = true; }
            else if (c.OfficerRole != "") { Why = "Officer · " + CardFormat.Sentence(UiText.Name(NameKind.Role, c.OfficerRole)); WhyIsProblem = false; }
            else if (c.BuildingId != "") { Why = "Works at the " + WorkplaceName(c); WhyIsProblem = false; }
            else { Why = ""; WhyIsProblem = false; }
        }

        protected override string Shape()
        {
            var c = Cat;
            var job = Village.Jobs.Find(j => j.Id == c.JobId);
            string items = string.Join(",", Village.Items.Where(i => i.LocationId == c.Id || job != null && i.LocationId == job.Id).Select(i => i.Id));
            // Carried exact items show once picked up, so the phase matters only while items travel with the job.
            return c.Alive + "|" + c.JobId + "|" + (items != "" ? job?.Phase : "") + "|" + (job != null && job.RequiredWork > 0 ? job.Id : "") + "|" + c.BuildingId + "|" + c.BedId + "|" + items + "|"
                + string.Join(",", c.Preferences) + "|" + c.Boosted + "|" + assignOpen + "|" + (assignOpen ? Candidates(Village, World, c).Count().ToString(CultureInfo.InvariantCulture) : "");
        }

        protected override void OnBind() => assignOpen = false;

        protected override void Build()
        {
            rings.Clear();
            work = null;
            var c = Cat;
            if (!c.Alive) return;
            var job = Village.Jobs.Find(j => j.Id == c.JobId);
            if (job != null && job.RequiredWork > 0) work = Progress(this, UiNames.WorkProgress);
            var row = UiKit.Element(this, "card-rings");
            foreach (var need in Needs)
            {
                var ring = new NeedRing(need, CardFormat.Sentence(need));
                rings[need] = ring;
                row.Add(ring);
            }

            var actions = Actions();
            // Help speeds up the task in hand (W2-06 lets Boost resolve a cat id to its station work, too).
            var help = Act(actions, "Help", UiNames.Act("help", c.Id), () => new GameAction { Kind = "Boost", TargetId = c.JobId != "" ? c.JobId : c.Id, CatId = c.Id }, primary: true);
            help.tooltip = "Lend a paw: the current task finishes sooner";
            help.SetEnabled(c.JobId != "" || c.BuildingId != "");
            Plain(actions, assignOpen ? "Assign ▾" : "Assign to…", UiNames.Act("assign-to", c.Id), () => { assignOpen = !assignOpen; Rebuild(); });
            var control = Plain(actions, "Control · Tab", UiNames.Act("control", c.Id), () => Context.Game.View?.EnterSelectedCat());
            control.SetEnabled(c.ControlledBy == "");
            MoreButton(actions);

            if (assignOpen) BuildAssign(c);
            BuildMore(c, job);
        }

        private void BuildAssign(Cat c)
        {
            var list = UiKit.Element(this, "card-list");
            var places = Candidates(Village, World, c).Take(5).ToList();
            if (places.Count == 0) UiKit.Label(list, "No workplace has a free spot right now.", "card-dim");
            foreach (var b in places)
            {
                var building = b;
                int crew = building.Slots.Count(s => s.CatId != ""), slots = BuildingCard.CrewSlots(Village, building);
                string label = CardFormat.BuildingName(Village, building) + " · " + crew + "/" + slots;
                Act(list, label, UiNames.Act("assign", c.Id, building.Id), () => new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = building.Id }).AddToClassList("card-row-act");
            }
            if (c.BuildingId != "")
                Act(list, "Release from the " + WorkplaceName(c), UiNames.Act("release-workplace", c.Id), () => new GameAction { Kind = "AssignWorker", CatId = c.Id }).AddToClassList("card-row-act");
            Plain(list, "Manual jobs in Work", UiNames.Act("open-work", "card"), () => Context.Hud.OpenPanel("Work")).AddToClassList("card-quiet");
        }

        private void BuildMore(Cat c, Job job)
        {
            var more = MoreArea();
            needLine = UiKit.Label(more, "", "card-dim");
            var kv = UiKit.Element(more, "card-kv");
            Row(kv, "Bed", c.BedId == "" ? "None yet" : CardFormat.BuildingName(Village, Village.Buildings.Find(b => b.Id == c.BedId) ?? new Building { Kind = "den" }));
            Row(kv, "Work", c.BuildingId == "" ? "No workplace" : WorkplaceName(c));
            Row(kv, "Skills", c.Skills.Count == 0 ? "Still learning" : string.Join(", ", c.Skills.OrderByDescending(s => s.Amount).Take(5).Select(s => CardFormat.Sentence(UiText.Name(NameKind.Labor, s.Resource)) + " " + CardFormat.Amount(s.Amount))));
            Row(kv, "Cargo", CardFormat.Goods(c.Cargo));
            var carried = job != null && !job.Completed && job.Phase != JobPhase.ItemFetch ? Village.Items.Where(i => i.LocationId == job.Id).ToList() : new List<Item>();
            foreach (var item in carried)
                Row(kv, "Carrying", ItemName(item) + " · " + CardFormat.Whole(item.Condition) + "% condition", "carried-item-" + item.Id);

            UiKit.Label(more, "Equipment", "card-subtitle");
            var equipment = Village.Items.Where(i => i.LocationId == c.Id).ToList();
            if (equipment.Count == 0) UiKit.Label(more, "Nothing equipped", "card-dim");
            foreach (var item in equipment)
            {
                var row = UiKit.Element(more, "card-line");
                UiKit.Label(row, ItemName(item) + " · " + CardFormat.Whole(item.Condition) + "%", "card-line-text");
                string id = item.Id;
                Act(row, "Unequip", UiNames.Act("unequip", id), () => new GameAction { Kind = "UnequipItem", CatId = c.Id, TargetId = id }).AddToClassList("card-small");
            }

            UiKit.Label(more, "Likes to do", "card-subtitle");
            var grid = UiKit.Element(more, "card-prefs");
            foreach (var labor in Catalog.Labors)
            {
                string id = labor;
                var toggle = new Toggle(CardFormat.Sentence(UiText.Name(NameKind.Labor, labor))) { value = c.Preferences.Contains(labor), name = UiNames.Act("labor", labor) };
                toggle.AddToClassList("card-pref");
                toggle.RegisterValueChangedCallback(e => _ = Send(new GameAction { Kind = "SetCatLaborPreference", CatId = c.Id, Labor = id, Enabled = e.newValue }));
                grid.Add(toggle);
            }
            var prefer = Act(more, c.Boosted ? "Preferred for work ✓" : "Prefer for work", UiNames.Act("prefer", c.Id), () => new GameAction { Kind = "BoostCat", CatId = c.Id, Enabled = !c.Boosted });
            prefer.AddToClassList("card-quiet");

            var events = Village.Events.Where(e => e.EntityId == c.Id).Skip(System.Math.Max(0, Village.Events.Count(e => e.EntityId == c.Id) - 6)).Reverse().ToList();
            if (events.Count > 0)
            {
                UiKit.Label(more, "Lately", "card-subtitle");
                foreach (var e in events) UiKit.Label(more, UiText.EventLine(e), "card-event");
            }
        }

        protected override void Update()
        {
            var c = Cat;
            if (!c.Alive) return;
            if (rings.Count == 4)
            {
                rings["hunger"].Set(c.Hunger);
                rings["thirst"].Set(c.Thirst);
                rings["rest"].Set(c.Rest);
                rings["health"].Set(c.Health);
            }
            var job = Village.Jobs.Find(j => j.Id == c.JobId);
            if (work != null && job != null)
            {
                double rate = job.RequiredWork > 0 ? job.Progress / job.RequiredWork : 0;
                SetProgress(work, job.Progress, job.RequiredWork, CardFormat.Sentence(UiText.Name(NameKind.Job, job.Kind)) + " · " + CardFormat.Whole(rate * 100) + "%");
            }
            if (needLine != null && MoreOpen)
                TetheredCard.SetText(needLine, "Hunger " + CardFormat.Whole(c.Hunger) + " · thirst " + CardFormat.Whole(c.Thirst) + " · rest " + CardFormat.Whole(c.Rest) + " · health " + CardFormat.Whole(c.Health));
        }

        private static string ItemName(Item item) => CardFormat.Sentence(UiText.Name(NameKind.Resource, item.Material) + " " + UiText.Name(NameKind.Item, item.Kind));

        private string WorkplaceName(Cat c)
        {
            var b = Village.Buildings.Find(x => x.Id == c.BuildingId);
            if (b != null) return CardFormat.BuildingName(Village, b);
            return Village.Farms.Any(f => f.Id == c.BuildingId) ? "field" : "workplace";
        }

        /// <summary>Completed workplaces with a free crew slot, best match first (preferred labor, then skill).</summary>
        internal static IEnumerable<Building> Candidates(Village v, World w, Cat c)
        {
            return v.Buildings
                .Where(b => b.Completed && b.Id != c.BuildingId && BuildingCard.IsWorkplace(b) && b.Slots.Count(s => s.CatId != "" && s.CatId != c.Id) < BuildingCard.CrewSlots(v, b))
                .OrderByDescending(b => (c.Preferences.Contains(BuildingCard.LaborOf(b.Kind)) ? 100 : 0) + Skill(c, BuildingCard.LaborOf(b.Kind)))
                .ThenBy(b => Int2.Distance(b.Position, c.Position));
        }

        internal static double Skill(Cat c, string labor) => c.Skills.Find(s => s.Resource == labor)?.Amount ?? 0;

        /// <summary>"Leader", an office ("Steward"), a trade from the cat's work ("Hauler", "Scholar") or "Villager".</summary>
        internal static string RoleOf(Village v, Cat c)
        {
            if (c.Id == v.LeaderId) return "Leader";
            if (c.OfficerRole != "") return CardFormat.Sentence(UiText.Name(NameKind.Role, c.OfficerRole));
            var job = v.Jobs.Find(j => j.Id == c.JobId);
            string trade = job != null ? TradeOf(job.Kind) : "";
            if (trade == "" && c.BuildingId != "")
            {
                var b = v.Buildings.Find(x => x.Id == c.BuildingId);
                trade = b != null ? TradeOf(BuildingCard.LaborOf(b.Kind)) : v.Farms.Any(f => f.Id == c.BuildingId) ? "Farmer" : "";
            }
            return trade != "" ? trade : "Villager";
        }

        private static string TradeOf(string work)
        {
            switch (work)
            {
                case "haul": return "Hauler";
                case "hunt": case "leader_plan_hunt": case "hunt_expedition": return "Hunter";
                case "water": case "fetch_water": case "supply_water": return "Water carrier";
                case "logs": case "woodcut": case "gather_logs": return "Woodcutter";
                case "quarry": return "Quarrier";
                case "fibre": case "forage": case "forage_fibre": return "Forager";
                case "fish": case "fishing": return "Fisher";
                case "scout": case "explore": return "Scout";
                case "build": case "build_house": case "expand": case "expand_village": case "road": return "Builder";
                case "research": return "Scholar";
                case "train": case "fight": case "train_warrior": return "Guard";
                case "offering": case "ritual": case "carry_offering": case "perform_offering": return "Acolyte";
                case "farm": case "mouse_farm": return "Farmer";
                case "production": case "process": case "craft": return "Crafter";
                case "mill": return "Miller";
                case "textile": return "Weaver";
                case "metalwork": return "Smith";
                default: return "";
            }
        }

        internal static string LifeStage(World w, Cat c)
        {
            var pace = w.CurrentPacing();
            if (c.AgeHours < pace.KittenUntilHours) return "kitten";
            if (c.AgeHours >= pace.HonouredElderAgeHours) return "honoured elder";
            if (c.AgeHours >= pace.ElderAgeHours) return "elder";
            return "adult";
        }
    }

    /// <summary>
    /// A need ring (GUIDE 5.5): a 50 px dial with a 6 px track, the value in the hole and the need's name below,
    /// coloured by value. Named <c>ring-&lt;need&gt;</c>; the tone class is <c>tone-ok|tone-sun|tone-bad</c>.
    /// </summary>
    public sealed class NeedRing : VisualElement
    {
        private static readonly Color Track = new Color(1, 1, 1, .12f);
        private readonly VisualElement dial;
        private readonly Label number;
        private int value = -1;
        private string tone = "";

        public NeedRing(string need, string caption)
        {
            name = "ring-" + need;
            AddToClassList("card-ring");
            dial = UiKit.Element(this, "card-ring-dial");
            dial.generateVisualContent += Draw;
            number = UiKit.Label(dial, "", "card-ring-number");
            UiKit.Label(this, caption, "card-ring-label");
        }

        public int Value => value;
        public string Tone => tone;

        /// <summary>"ok" at 60 and above, "sun" from 35 to 59, "bad" below 35.</summary>
        public static string ToneOf(double v) => v >= 60 ? "ok" : v >= 35 ? "sun" : "bad";

        public void Set(double v)
        {
            int n = Mathf.Clamp(Mathf.RoundToInt((float)v), 0, 100);
            if (n == value) return;
            value = n;
            number.text = n.ToString(CultureInfo.InvariantCulture);
            string next = ToneOf(n);
            if (next != tone)
            {
                if (tone != "") RemoveFromClassList("tone-" + tone);
                tone = next;
                AddToClassList("tone-" + tone);
            }
            dial.MarkDirtyRepaint();
        }

        private void Draw(MeshGenerationContext context)
        {
            var rect = dial.contentRect;
            float radius = Mathf.Min(rect.width, rect.height) * .5f - 3;
            if (radius <= 0) return;
            var painter = context.painter2D;
            painter.lineWidth = 6;
            painter.strokeColor = Track;
            painter.BeginPath();
            painter.Arc(rect.center, radius, Angle.Degrees(0), Angle.Degrees(360));
            painter.Stroke();
            if (value <= 0) return;
            painter.strokeColor = tone == "ok" ? new Color(.49f, .83f, .48f) : tone == "sun" ? new Color(1f, .82f, .4f) : new Color(1f, .42f, .34f);
            painter.lineCap = LineCap.Butt;
            painter.BeginPath();
            painter.Arc(rect.center, radius, Angle.Degrees(-90), Angle.Degrees(-90 + 360 * value / 100f));
            painter.Stroke();
        }
    }
}
