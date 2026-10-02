using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class VillageSection : ISection
    {
        public string Id => "Village";
        public string Label => "Village";
        public string Group => "Colony";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var game = ctx.Game;
            var village = ctx.Village;
            var picks = ctx.Picks;
            UiKit.Text(root, "The Commons is shared by everyone. Found a personal village to manage a home of your own.");
            foreach (var v in ctx.World.Villages)
            {
                string id = v.Id;
                UiKit.Button(root, v.Name + (v.Id == village.Id ? " · selected" : ""), () => ctx.Act(new GameAction { Kind = "JoinVillage", TargetId = id }, true)).Named(UiNames.Act("join", id));
            }
            var name = UiKit.Field(root, "New village name", "Mosslight").Named("field-village-name");
            UiKit.Button(root, "Found personal village", () => ctx.Act(new GameAction { Kind = "FoundVillage", Name = name.value }, true)).Named(UiNames.Act("found", "village"));
            UiKit.Separator(root);
            UiKit.Text(root, "Shared server");
            var server = UiKit.Field(root, "Server address", picks.ServerAddress).Named("field-server-address");
            server.RegisterValueChangedCallback(e => picks.ServerAddress = e.newValue);
            UiKit.Button(root, "Connect", () => { _ = game.Connect(picks.ServerAddress); }).Named(UiNames.Act("connect", "server"));
            if (game.IsRemote) UiKit.Button(root, "Return to local world", game.Disconnect).Named(UiNames.Act("disconnect", "server"));
            UiKit.Button(root, "Save local world", game.Save).Named(UiNames.Act("save", "world"));
            UiKit.Text(root, game.IsRemote ? "Your shared village continues on the server when you leave." : "This world saves on your Mac as you play and when you quit.", "muted");
            UiKit.Separator(root);
            UiKit.Label(root, "Governance", "subtitle");
            if (village.Election != null)
            {
                UiKit.Text(root, "Election open until hour " + (village.Election.EndAt / 3600).ToString("0.0"));
                foreach (var c in village.Cats.Where(c => c.Alive))
                {
                    string id = c.Id;
                    UiKit.Button(root, "Vote for " + c.Name, () => ctx.Act(new GameAction { Kind = "CastVote", CatId = id })).Named(UiNames.Act("vote", id));
                }
            }
            else UiKit.Text(root, "Next election at hour " + (village.NextElection / 3600).ToString("0.0"));
            if (village.KickPetition != null)
                UiKit.Text(root, village.KickPetition.Votes.Count + " / 5 signatures for leader removal · closes at hour " + (village.KickPetition.EndAt / 3600).ToString("0.0"));
            UiKit.Button(root, "Sign leader removal petition", () => ctx.Act(new GameAction { Kind = "RequestVoteKick" })).Named(UiNames.Act("sign", "petition"));
            return root;
        }
    }
}
