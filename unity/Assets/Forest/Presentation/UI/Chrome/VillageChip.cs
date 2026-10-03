using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The village chip (GUIDE §5.1), top left: the colony name in Instrument Serif, then the clock, the
    /// census (alive, at work and a clickable idle count) and "paused", and the speed segment (pause plus
    /// <see cref="GameSpeeds.Player"/>, and the dev speeds when enabled). It binds Space and 1, 2, 3 on the
    /// input router. Mounted in slot-chip.
    /// </summary>
    public class VillageChip : VisualElement, IHudPart
    {
        public const string ChipName = "village-chip";
        public const string NameLabel = "chip-name";
        public const string ClockLabel = "hud-clock";
        public const string CensusLabel = "chip-census";
        public const string IdleButton = "act-show-idle";
        public const string PausedLabel = "chip-paused";
        public const string SegmentName = "chip-speed";

        private static readonly CatActivity[] IdleActivities = { CatActivity.Idle, CatActivity.Nap, CatActivity.Groom, CatActivity.Social, CatActivity.Patrol, CatActivity.Play, CatActivity.Walk };

        private readonly Label colonyName, clock, census, paused;
        private readonly Button idle;
        private readonly VisualElement segment;
        private readonly List<(float speed, Button button)> speedButtons = new List<(float, Button)>();
        private float lastRunningSpeed = 1;

        public VillageChip(UiContext ctx)
        {
            Context = ctx;
            name = ChipName;
            AddToClassList("chip");
            AddToClassList("glass");
            var text = UiKit.Element(this, "chip-text");
            text.pickingMode = PickingMode.Ignore;
            colonyName = UiKit.Label(text, "", "chip-name").Named(NameLabel);
            var line = UiKit.Element(text, "chip-line");
            line.pickingMode = PickingMode.Ignore;
            clock = UiKit.Label(line, "", "chip-small").Named(ClockLabel);
            census = UiKit.Label(line, "", "chip-small").Named(CensusLabel);
            census.AddToClassList("chip-census");
            idle = UiKit.Button(line, "", () => ctx.Hud.OpenPanel("Cats")).Named(IdleButton);
            idle.AddToClassList("chip-idle");
            paused = UiKit.Label(line, "· paused", "chip-small").Named(PausedLabel);
            paused.AddToClassList("chip-paused");
            segment = UiKit.Element(this, "seg").Named(SegmentName);
            foreach (float speed in Speeds())
            {
                float selected = speed;
                var button = new Button(() => Choose(selected)) { name = UiNames.Act("speed", speed.ToString(CultureInfo.InvariantCulture)) };
                button.AddToClassList("seg-btn");
                if (speed == GameSpeeds.Paused)
                {
                    button.text = "";
                    button.AddToClassList("seg-pause");
                    button.Add(HudIcons.Glyph("pause", "seg-icon", "II"));
                }
                else button.text = speed.ToString(CultureInfo.InvariantCulture) + "×";
                segment.Add(button);
                speedButtons.Add((speed, button));
            }
            Refresh();
        }

        public UiContext Context { get; }
        private ForestGame Game => Context.Game;

        /// <summary>Pause, the player speeds and, in debug builds or with --forest-dev-speeds, the dev speeds.</summary>
        public static IEnumerable<float> Speeds()
        {
            yield return GameSpeeds.Paused;
            foreach (float s in GameSpeeds.Player) yield return s;
            if (GameSpeeds.DevSpeedsEnabled)
                foreach (float s in GameSpeeds.Dev) yield return s;
        }

        /// <summary>A cat counts as at work with a job or a workplace (station workers have no job id).</summary>
        public static bool IsWorking(Cat c) => c.Alive && (c.JobId != "" || c.BuildingId != "");

        /// <summary>Adult residents with nothing to do: no job, no workplace, no office, not busy with a need.</summary>
        public static int IdleCount(World world, Village v)
        {
            if (world == null || v == null) return 0;
            double kittenHours = world.CurrentPacing().KittenUntilHours;
            int count = 0;
            foreach (var c in v.Cats)
            {
                if (!c.Alive || IsWorking(c) || c.OfficerRole != "" || c.Id == v.LeaderId || c.AgeHours < kittenHours || c.Migration != "resident" || c.ControlledBy != "") continue;
                if (Array.IndexOf(IdleActivities, world.ActivityOf(v, c)) >= 0) count++;
            }
            return count;
        }

        /// <summary>Space: pause, or resume at the last running speed.</summary>
        public bool TogglePause()
        {
            if (Game.IsRemote || Context.Village == null) return false;
            if (Game.Speed == GameSpeeds.Paused) Game.SetSpeed(lastRunningSpeed);
            else { lastRunningSpeed = Game.Speed; Game.SetSpeed(GameSpeeds.Paused); }
            Refresh();
            return true;
        }

        /// <summary>1, 2 and 3 pick the player speeds in order.</summary>
        public bool SelectPlayerSpeed(int index)
        {
            if (Game.IsRemote || Context.Village == null || index < 0 || index >= GameSpeeds.Player.Count) return false;
            Game.SetSpeed(GameSpeeds.Player[index]);
            Refresh();
            return true;
        }

        public IDisposable Bind(InputRouter router)
        {
            var bindings = new UiState.Bindings();
            bindings.Add(router.Bind(InputIntent.TogglePause, InputPriority.World, TogglePause));
            bindings.Add(router.Bind(InputIntent.Speed1, InputPriority.World, () => SelectPlayerSpeed(0)));
            bindings.Add(router.Bind(InputIntent.Speed2, InputPriority.World, () => SelectPlayerSpeed(1)));
            bindings.Add(router.Bind(InputIntent.Speed3, InputPriority.World, () => SelectPlayerSpeed(2)));
            return bindings;
        }

        private void Choose(float speed)
        {
            if (Game.IsRemote) return;
            float current = Game.Speed;
            if (speed == GameSpeeds.Paused && current == GameSpeeds.Paused) speed = lastRunningSpeed;
            else if (speed == current && Context.Hud.IsNarrow)
            {
                // Narrow windows show pause and the current speed only; clicking the speed steps through them.
                var running = Speeds().Where(s => s > 0).ToList();
                speed = running[(running.IndexOf(current) + 1) % running.Count];
            }
            if (current > 0) lastRunningSpeed = current;
            Game.SetSpeed(speed);
            Refresh();
        }

        public void Refresh()
        {
            var village = Context.Village;
            var world = Context.World;
            if (village == null || world == null)
            {
                Set(colonyName, Game.Status);
                Set(clock, "");
                census.EnableInClassList("gone", true);
                idle.EnableInClassList("gone", true);
                paused.style.display = DisplayStyle.None;
                return;
            }
            float speed = Game.Speed;
            if (speed > 0) lastRunningSpeed = speed;
            Set(colonyName, village.Name);
            Set(clock, UiText.Clock(world.TimeSeconds, world.CurrentPacing().DayScale) + (Game.IsRemote ? " · shared" : ""));
            int alive = village.Cats.Count(c => c.Alive);
            int working = village.Cats.Count(IsWorking);
            int idleCats = IdleCount(world, village);
            Set(census, "· " + UiText.Plural(alive, "cat", "cats") + ", " + working + " at work" + (idleCats > 0 ? "," : ""));
            census.EnableInClassList("gone", false);
            Set(idle, idleCats + " idle");
            idle.EnableInClassList("gone", idleCats == 0);
            bool isPaused = !Game.IsRemote && speed == GameSpeeds.Paused;
            paused.style.display = isPaused ? DisplayStyle.Flex : DisplayStyle.None;
            EnableInClassList("paused", isPaused);
            foreach (var (value, button) in speedButtons)
            {
                bool on = !Game.IsRemote && value == speed;
                button.EnableInClassList("on", on);
                button.EnableInClassList("active", on);
                button.EnableInClassList("not-current", !on && value != GameSpeeds.Paused);
                button.SetEnabled(!Game.IsRemote);
            }
        }

        private static void Set(TextElement element, string text)
        {
            if (element.text != text) element.text = text;
        }
    }
}
