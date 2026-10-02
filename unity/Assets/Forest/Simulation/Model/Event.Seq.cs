namespace IdleCatForest.Simulation
{
    public partial class Event
    {
        /// <summary>
        /// World-unique, strictly increasing number assigned by <c>Note</c>, so the presentation can
        /// tell which events are new (Time is not unique). 0 means written before sequencing existed.
        /// </summary>
        public long Seq;
    }

    public partial class World
    {
        /// <summary>The sequence number the next event receives. Old saves load with 0.</summary>
        public long NextEventSeq;

        private long TakeEventSeq()
        {
            // Continue above the highest number already in any log, so a save without this field
            // (or with a stale one) never reuses a sequence number.
            long highest = 0;
            foreach (var village in Villages)
                foreach (var e in village.Events)
                    if (e.Seq > highest)
                        highest = e.Seq;
            if (NextEventSeq <= highest)
                NextEventSeq = highest + 1;
            return NextEventSeq++;
        }
    }
}
