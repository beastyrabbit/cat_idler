using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class Village
    {
        /// <summary>
        /// Info-severity events (job completions, scout notes, repairs) in their own capped ring, so
        /// routine noise never evicts the notable and alert events kept in <c>Events</c>.
        /// </summary>
        public List<Event> RoutineEvents = new List<Event>();
    }

    public partial class World
    {
        /// <summary>Most notable and alert events a village keeps (the original log cap).</summary>
        public const int NotableEventCap = 100;
        /// <summary>Most routine (info) events a village keeps.</summary>
        public const int RoutineEventCap = 100;
        /// <summary>A repeated error for the same entity is logged at most once per this many game seconds.</summary>
        public const double ErrorRepeatSeconds = 60;
        /// <summary>Storage full and empty notices for the same resource repeat at most once per this many game seconds.</summary>
        public const double StorageNoticeSeconds = 120;

        private void Note(Village v, string kind, string text, string id = "") => NoteEvent(v, kind, text, id, null, "", null);

        /// <summary>
        /// Writes one event. Position and actor come from the entity (cat, job, building, raid, pile or
        /// caravan) unless given; the severity defaults to the kind's (<see cref="EventSeverity.ForKind"/>).
        /// Info events go to <see cref="Village.RoutineEvents"/>, everything else to <see cref="Village.Events"/>.
        /// </summary>
        private Event NoteEvent(Village v, string kind, string text, string entityId, Int2? position, string severity, Dictionary<string, string> args)
        {
            entityId = entityId ?? "";
            var e = new Event { Time = TimeSeconds, Kind = kind ?? "", Text = text ?? "", EntityId = entityId, Severity = string.IsNullOrEmpty(severity) ? EventSeverity.ForKind(kind) : severity, Args = args };
            DescribeEntity(v, entityId, e);
            if (position.HasValue)
                e.Position = position;
            foreach (var village in Villages)
                foreach (var routine in village.RoutineEvents)
                    if (routine.Seq >= NextEventSeq)
                        NextEventSeq = routine.Seq + 1;
            e.Seq = TakeEventSeq();
            if (e.Severity == EventSeverity.Info)
                Push(v.RoutineEvents, e, RoutineEventCap);
            else
                Push(v.Events, e, NotableEventCap);
            return e;
        }

        private static void Push(List<Event> log, Event e, int cap)
        {
            log.Add(e);
            while (log.Count > cap)
            {
                // Logs saved before the split still hold routine entries: those go first.
                int victim = log.FindIndex(x => x.EffectiveSeverity() == EventSeverity.Info);
                log.RemoveAt(victim >= 0 && victim < log.Count - 1 ? victim : 0);
            }
        }

        private void DescribeEntity(Village v, string id, Event e)
        {
            if (id == "" || v == null)
                return;
            var cat = v.Cats.Find(c => c.Id == id);
            if (cat != null)
            {
                e.Position = cat.Position;
                e.ActorName = cat.Name;
                return;
            }
            var job = v.Jobs.Find(j => j.Id == id);
            if (job != null)
            {
                e.Position = job.Position;
                e.ActorName = v.Cats.Find(c => c.Id == job.CatId)?.Name ?? "";
                e.Args = e.Args ?? new Dictionary<string, string>();
                if (!e.Args.ContainsKey("job"))
                    e.Args["job"] = job.Kind;
                return;
            }
            var building = v.Buildings.Find(b => b.Id == id);
            if (building != null)
            {
                e.Position = building.Position;
                return;
            }
            var raid = v.Raids.Find(r => r.Id == id);
            if (raid != null)
            {
                e.Position = raid.Position;
                return;
            }
            var pile = v.Stockpiles.Find(s => s.Id == id);
            if (pile != null)
            {
                e.Position = pile.Position;
                return;
            }
            var trade = TradeOffers.Find(t => t.Id == id);
            if (trade != null)
                e.Position = trade.Position;
        }

        /// <summary>
        /// Every event of the village with a sequence number above <paramref name="afterSeq"/>, oldest first,
        /// from both the notable and the routine log. Pass 0 for the whole log (events saved before
        /// sequencing have Seq 0 and are only returned then).
        /// </summary>
        public List<Event> EventsSince(Village v, long afterSeq)
        {
            var result = new List<Event>();
            if (v == null)
                return result;
            foreach (var e in v.Events)
                if (e.Seq > afterSeq || afterSeq <= 0)
                    result.Add(e);
            foreach (var e in v.RoutineEvents)
                if (e.Seq > afterSeq || afterSeq <= 0)
                    result.Add(e);
            return result.OrderBy(e => e.Seq).ToList();
        }

        /// <summary>A notable event with explicit metadata; the generic form of the helpers below.</summary>
        public Event NoteMilestone(Village v, string kind, string text, string entityId = "", Int2? position = null, params (string key, string value)[] args) =>
            v == null ? null : NoteEvent(v, kind, text, entityId, position, EventSeverity.Notable, ToArgs(args));

        public Event NoteBuildingCompleted(Village v, Building building) =>
            building == null ? null : NoteMilestone(v, EventKind.BuildingCompleted, Words(building.Kind) + " completed", building.Id, building.Position, ("building", building.Kind));

        public Event NoteResourceDiscovered(Village v, Cat finder, string resource, Int2 at) =>
            NoteMilestone(v, EventKind.Discovery, (finder == null ? "Scouts" : finder.Name) + " found " + Words(resource), finder?.Id ?? "", at, ("resource", resource ?? ""));

        public Event NoteStorageFull(Village v, Stockpile pile, string resource)
        {
            if (v == null || RecentlyNoted(v, EventKind.StorageFull, resource, StorageNoticeSeconds))
                return null;
            return NoteMilestone(v, EventKind.StorageFull, "Storage is full of " + Words(resource), pile?.Id ?? "", pile?.Position, ("resource", resource ?? ""));
        }

        public Event NoteStorageEmpty(Village v, string resource)
        {
            if (v == null || RecentlyNoted(v, EventKind.StorageEmpty, resource, StorageNoticeSeconds))
                return null;
            return NoteEvent(v, EventKind.StorageEmpty, "Stores ran out of " + Words(resource), "", null, EventSeverity.Alert, ToArgs(new[] { ("resource", resource ?? "") }));
        }

        public Event NoteRankUp(Village v, Cat cat, string skill, int rank) =>
            cat == null ? null : NoteMilestone(v, EventKind.RankUp, cat.Name + " reached rank " + rank.ToString(CultureInfo.InvariantCulture) + " in " + Words(skill), cat.Id, null, ("skill", skill ?? ""), ("rank", rank.ToString(CultureInfo.InvariantCulture)));

        public Event NoteRaidRepelled(Village v, Raid raid) =>
            raid == null ? null : NoteMilestone(v, EventKind.RaidRepelled, "Raiders were driven off", raid.Id, raid.Position);

        public Event NoteExpeditionLoot(Village v, Cat cat, IEnumerable<Stack> loot)
        {
            var stacks = (loot ?? Enumerable.Empty<Stack>()).Where(s => s != null && s.Amount > 0).ToList();
            var args = stacks.Select(s => (s.Resource, Number(s.Amount))).ToArray();
            string text = (cat == null ? "The expedition" : cat.Name) + " brought home " + (stacks.Count == 0 ? "nothing" : string.Join(", ", stacks.Select(s => Number(s.Amount) + " " + Words(s.Resource))));
            return NoteMilestone(v, EventKind.ExpeditionLoot, text, cat?.Id ?? "", null, args);
        }

        private bool RecentlyNoted(Village v, string kind, string resource, double seconds)
        {
            for (int i = v.Events.Count - 1; i >= 0; i--)
            {
                var e = v.Events[i];
                if (TimeSeconds - e.Time >= seconds)
                    break;
                if (e.Kind == kind && e.Args != null && e.Args.TryGetValue("resource", out var noted) && noted == (resource ?? ""))
                    return true;
            }
            return false;
        }

        private static Dictionary<string, string> ToArgs((string key, string value)[] args)
        {
            if (args == null || args.Length == 0)
                return null;
            var map = new Dictionary<string, string>();
            foreach (var (key, value) in args)
                if (!string.IsNullOrEmpty(key))
                    map[key] = value ?? "";
            return map.Count == 0 ? null : map;
        }

        private static string Words(string id) => (id ?? "").Replace('_', ' ');
        private static string Number(double amount) => Math.Round(amount, 2).ToString("0.##", CultureInfo.InvariantCulture);
    }
}
