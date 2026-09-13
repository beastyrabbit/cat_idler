using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;

namespace IdleCatForest.Authority
{
    [Serializable]
    public sealed class ClientRequest
    {
        public int ProtocolVersion = 3;
        public string Type = "action", RequestId = "", SessionId = "", Sig = "";
        public GameAction Action;
    }
    [Serializable]
    public sealed class ServerMessage
    {
        public int ProtocolVersion = 3;
        public string Type = "snapshot", RequestId = "", Error = "";
        public SessionCredential Credential;
        public ActionResult Result;
        public WorldFrame Frame;
    }
    [Serializable]
    public sealed class WorldFrame
    {
        public long Sequence;
        public World World;
        public string SelectedVillageId = "";
        public List<string> ControlledVillageIds = new List<string>();
        public List<VillageSummary> KnownVillages = new List<VillageSummary>();
    }
    [Serializable]
    public sealed class VillageSummary
    {
        public string Id = "", Name = "";
        public Int2 Center;
        public bool Communal, CanControl;
    }
}
