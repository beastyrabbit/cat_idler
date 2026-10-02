using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// The single entry point for player actions: shared validation, kind normalisation and
        /// identity resolution, then one handler per kind in the Actions.&lt;Domain&gt;.cs files.
        /// </summary>
        public ActionResult Apply(PlayerContext context, GameAction action)
        {
            if (context == null || action == null)
                return ActionResult.Fail("Missing action or identity");
            if (action.Position.Level != 0 || action.End.Level != 0 || action.Path != null && action.Path.Any(p => p.Level != 0))
                return ActionResult.Fail("Surface designations require surface coordinates; use the dungeon stairs to change level");
            if (!Finite(action.Amount) || !Finite(action.OtherAmount) || Math.Abs((long)action.Position.X) > 1000000 || Math.Abs((long)action.Position.Z) > 1000000 || Math.Abs((long)action.End.X) > 1000000 || Math.Abs((long)action.End.Z) > 1000000 || action.Accepts == null || action.Path == null || action.Path.Count > 1024 || action.Path.Any(p => Math.Abs((long)p.X) > 1000000 || Math.Abs((long)p.Z) > 1000000))
                return ActionResult.Fail("Invalid finite action arguments");
            string kind = (action.Kind ?? "").Replace("_", "").ToLowerInvariant();
            if (kind == "ensure")
                return ActionResult.Ok();
            if (string.IsNullOrWhiteSpace(context.PlayerId))
                return ActionResult.Fail("Authenticated identity required");
            var player = Players.Find(p => p.Id == context.PlayerId);
            if (player == null)
            {
                player = new Player { Id = context.PlayerId };
                Players.Add(player);
            }
            if (kind == "presence")
                return ActionResult.Ok();
            if (kind == "foundvillage")
                return HandleFoundVillage(context, player, action);
            if (kind == "joinvillage")
                return HandleJoinVillage(context, player, action);
            var village = Village(string.IsNullOrEmpty(context.VillageId) ? player.SelectedVillageId : context.VillageId);
            if (!CanControl(context, village))
                return ActionResult.Fail("Village ownership denied");
            var cat = village.Cats.Find(c => c.Id == action.CatId && c.Alive);
            var building = village.Buildings.Find(b => b.Id == (string.IsNullOrEmpty(action.BuildingId) ? action.TargetId : action.BuildingId));
            switch (kind)
            {
                case "settestacceleration":
                case "advancetime":
                case "settestrngseed":
                    return HandleDeveloperClock(context, action, kind);
                case "requestjob":
                    return HandleRequestJob(village, action, cat);
                case "dispatchscout":
                    return HandleDispatchScout(village, action, cat);
                case "boost":
                    return HandleBoost(village, action);
                case "boostcat":
                    return HandleBoostCat(action, cat);
                case "setcatlaborpreference":
                    return HandleSetCatLaborPreference(action, cat);
                case "planbuilding":
                    return HandlePlanBuilding(village, action, cat);
                case "assignworker":
                    return HandleAssignWorker(village, action, cat, building);
                case "assignofficer":
                    return HandleAssignOfficer(village, action, cat);
                case "unassignofficer":
                    return HandleUnassignOfficer(village, action);
                case "researchnode":
                case "unlocknode":
                    return HandleResearchNode(village, action, kind);
                case "purchaseupgrade":
                    return HandlePurchaseUpgrade(village, action);
                case "editproductionqueue":
                case "editproductionworkslot":
                    return HandleEditProductionQueue(village, action, kind, cat, building);
                case "designatestockpile":
                case "designategatherspot":
                case "designatefishingspot":
                    return HandleDesignatePile(village, action, kind);
                case "removestockpile":
                case "removegatherspot":
                    return HandleRemovePile(village, action);
                case "haulgatherspot":
                    return HandleHaulGatherSpot(village, action, cat);
                case "designatefarm":
                    return HandleDesignateFarm(village, action);
                case "clearfarm":
                    return HandleClearFarm(village, action);
                case "createzone":
                    return HandleCreateZone(village, action);
                case "removezone":
                    return HandleRemoveZone(village, action);
                case "offertithe":
                    return HandleOfferTithe(village);
                case "offermaterials":
                case "offerresource":
                    return HandleOfferResource(village, action, kind, cat);
                case "trainwarrior":
                    return HandleTrainWarrior(village, cat);
                case "defendraid":
                    return HandleDefendRaid(village);
                case "castvote":
                    return HandleCastVote(context, village, cat);
                case "requestvotekick":
                    return HandleRequestVoteKick(context, village);
                case "buildroad":
                case "designaterail":
                    return HandleBuildRoad(village, action, kind, cat);
                case "buildbridge":
                    return HandleBuildBridge(village, action, cat);
                case "builddock":
                    return HandleBuildDock(village, action, cat);
                case "buildtransportvehicle":
                    return HandleBuildTransportVehicle(village, action, cat);
                case "createtransportroute":
                    return HandleCreateTransportRoute(village, action, cat);
                case "canceltransportroute":
                    return HandleCancelTransportRoute(village, action);
                case "offervillagetrade":
                    return HandleOfferVillageTrade(village, action);
                case "acceptvillagetrade":
                    return HandleAcceptVillageTrade(village, action);
                case "cancelvillagetrade":
                    return HandleCancelVillageTrade(village, action);
                case "buyresource":
                    return HandleBuyResource(village, action);
                case "sellgoods":
                    return HandleSellGoods(village, action);
                case "equipitem":
                case "unequipitem":
                case "repairitem":
                    return HandleEquipment(village, action, kind, cat);
                case "entercatcontrol":
                case "entercat":
                    return HandleEnterCatControl(context, village, cat);
                case "leavecatcontrol":
                case "leavecat":
                    return HandleLeaveCatControl(context, cat);
                case "keepcatcontrol":
                    return HandleKeepCatControl(context, cat);
                case "movecat":
                    return HandleMoveCat(context, village, action, cat);
                case "interactcat":
                    return HandleInteractCat(context, village, action, cat);
                case "exploredungeon":
                    return HandleExploreDungeon(village, action, cat);
                case "recallexplorer":
                    return HandleRecallExplorer(village, cat);
                case "attackcreature":
                    return HandleAttackCreature(context, village, action, cat);
                default:
                    return ActionResult.Fail("Unknown action: " + action.Kind);
            }
        }
        private ActionResult HandleDeveloperClock(PlayerContext context, GameAction action, string kind)
        {
            if (!context.IsDeveloper)
                return ActionResult.Fail("Developer control disabled");
            if (kind == "advancetime")
                Step(Math.Clamp(action.Amount, 0, 86400));
            else if (kind == "settestrngseed")
                RandomState = unchecked((uint)action.Amount);
            else
                return ActionResult.Fail("Explicit AdvanceTime replaces implicit accelerated clocks");
            return ActionResult.Ok();
        }
        public static string Snake(string value) => string.Concat((value ?? "").Select((ch, i) => char.IsUpper(ch) ? (i > 0 ? "_" : "") + char.ToLowerInvariant(ch) : ch.ToString())).Replace(" ", "_");
    }
}
