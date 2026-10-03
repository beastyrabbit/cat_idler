using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Every normalised action kind <see cref="Apply"/> accepts (aliases included), in router order.</summary>
        public static readonly IReadOnlyList<string> ActionKinds = new[]
        {
            "ensure", "presence", "foundvillage", "joinvillage",
            "settestacceleration", "advancetime", "settestrngseed",
            "requestjob", "dispatchscout", "boost", "boostcat", "preferforwork", "setcatlaborpreference", "planbuilding",
            "assignworker", "releaseworker", "assignofficer", "unassignofficer", "researchnode", "unlocknode", "hurrystudy",
            "purchaseupgrade", "editproductionqueue", "editproductionworkslot", "designatestockpile", "designategatherspot",
            "designatefishingspot", "removestockpile", "removegatherspot", "haulgatherspot", "designatefarm", "clearfarm",
            "createzone", "removezone", "offertithe", "offermaterials", "offerresource", "trainwarrior", "defendraid",
            "castvote", "requestvotekick", "buildroad", "designaterail", "buildbridge", "builddock", "buildtransportvehicle",
            "createtransportroute", "canceltransportroute", "offervillagetrade", "acceptvillagetrade", "cancelvillagetrade",
            "buyresource", "sellgoods", "equipitem", "unequipitem", "repairitem", "entercatcontrol", "entercat",
            "leavecatcontrol", "leavecat", "keepcatcontrol", "movecat", "stopcat", "interactcat", "exploredungeon",
            "recallexplorer", "attackcreature",
        };

        /// <summary>Kinds that need other players: governance, petitions, founding and inter-village trade.</summary>
        private static readonly string[] MultiplayerKinds = { "castvote", "requestvotekick", "foundvillage", "offervillagetrade", "acceptvillagetrade", "cancelvillagetrade" };

        /// <summary>Distinct signatures a petition needs to pass (see Systems/Governance.cs).</summary>
        private const int PetitionSignatures = 5;

        /// <summary>Lower-case kind without underscores, the form the router and <see cref="ActionKinds"/> use.</summary>
        public static string NormaliseKind(string kind) => (kind ?? "").Replace("_", "").ToLowerInvariant();

        /// <summary>
        /// Whether a kind makes sense in this world for this player, without applying it. Governance,
        /// petitions, founding and inter-village trade return <see cref="ResultCode.RequiresPlayers"/>
        /// while the world has a single player identity (petitions until five players exist; trade until
        /// another player owns a village). Advisory: the UI hides such actions; <see cref="Apply"/> keeps
        /// its own rules so shared worlds behave as before.
        /// </summary>
        public ActionResult ActionAvailable(PlayerContext context, string kind)
        {
            string k = NormaliseKind(kind);
            if (!ActionKinds.Contains(k))
                return ActionResult.Fail(ResultCode.UnknownAction, "Unknown action: " + kind, (ResultArg.Kind, kind ?? ""));
            if (!MultiplayerKinds.Contains(k))
                return ActionResult.Ok(ResultCode.Ok, "");
            string me = context?.PlayerId ?? "";
            int players = Players.Select(p => p.Id).Append(me).Where(id => !string.IsNullOrEmpty(id)).Distinct().Count();
            int needed = k == "requestvotekick" ? PetitionSignatures : 2;
            bool otherVillage = Villages.Any(v => !v.Communal && v.OwnerId != "" && v.OwnerId != me);
            if (players < needed || k.EndsWith("villagetrade", StringComparison.Ordinal) && !otherVillage)
                return ActionResult.Fail(ResultCode.RequiresPlayers, "This needs other players in a shared world", (ResultArg.Kind, k), (ResultArg.Need, ActionResult.Number(needed)), (ResultArg.Have, ActionResult.Number(players)));
            return ActionResult.Ok(ResultCode.Ok, "");
        }

        /// <summary>
        /// The single entry point for player actions: shared validation, kind normalisation and
        /// identity resolution, then one handler per kind in the Actions.&lt;Domain&gt;.cs files. Every
        /// result carries a non-empty <see cref="ActionResult.Code"/>.
        /// </summary>
        public ActionResult Apply(PlayerContext context, GameAction action)
        {
            var result = Route(context, action);
            // Handlers outside W2-06's files may still return uncoded legacy results.
            return result.Coded(result.Success ? ResultCode.Ok : ResultCode.Rejected);
        }

        private ActionResult Route(PlayerContext context, GameAction action)
        {
            if (context == null || action == null)
                return ActionResult.Fail(ResultCode.InvalidAction, "Missing action or identity");
            if (action.Position.Level != 0 || action.End.Level != 0 || action.Path != null && action.Path.Any(p => p.Level != 0))
                return ActionResult.Fail(ResultCode.SurfaceOnly, "Surface designations require surface coordinates; use the dungeon stairs to change level");
            if (!Finite(action.Amount) || !Finite(action.OtherAmount) || Math.Abs((long)action.Position.X) > 1000000 || Math.Abs((long)action.Position.Z) > 1000000 || Math.Abs((long)action.End.X) > 1000000 || Math.Abs((long)action.End.Z) > 1000000 || action.Accepts == null || action.Path == null || action.Path.Count > 1024 || action.Path.Any(p => Math.Abs((long)p.X) > 1000000 || Math.Abs((long)p.Z) > 1000000))
                return ActionResult.Fail(ResultCode.InvalidArguments, "Invalid finite action arguments");
            string kind = NormaliseKind(action.Kind);
            if (kind == "ensure")
                return ActionResult.Ok(ResultCode.Ok, "");
            if (string.IsNullOrWhiteSpace(context.PlayerId))
                return ActionResult.Fail(ResultCode.IdentityRequired, "Authenticated identity required");
            var player = Players.Find(p => p.Id == context.PlayerId);
            if (player == null)
            {
                player = new Player { Id = context.PlayerId };
                Players.Add(player);
            }
            if (kind == "presence")
                return ActionResult.Ok(ResultCode.Ok, "");
            if (kind == "foundvillage")
                return HandleFoundVillage(context, player, action);
            if (kind == "joinvillage")
                return HandleJoinVillage(context, player, action);
            var village = Village(string.IsNullOrEmpty(context.VillageId) ? player.SelectedVillageId : context.VillageId);
            if (!CanControl(context, village))
                return ActionResult.Fail(ResultCode.VillageDenied, "Village ownership denied");
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
                case "preferforwork":
                    return HandleBoostCat(action, cat);
                case "setcatlaborpreference":
                    return HandleSetCatLaborPreference(action, cat);
                case "planbuilding":
                    return HandlePlanBuilding(village, action, cat);
                case "assignworker":
                    return HandleAssignWorker(village, action, cat, building);
                case "releaseworker":
                    return HandleReleaseWorker(village, action, cat);
                case "assignofficer":
                    return HandleAssignOfficer(village, action, cat);
                case "unassignofficer":
                    return HandleUnassignOfficer(village, action);
                case "researchnode":
                case "unlocknode":
                    return HandleResearchNode(village, action, kind);
                case "hurrystudy":
                    return HandleHurryStudy(village, action);
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
                    return HandleLeaveCatControl(context, village, cat);
                case "keepcatcontrol":
                    return HandleKeepCatControl(context, cat);
                case "movecat":
                    return HandleMoveCat(context, village, action, cat);
                case "stopcat":
                    return HandleStopCat(context, cat);
                case "interactcat":
                    return HandleInteractCat(context, village, action, cat);
                case "exploredungeon":
                    // Checked here so a refusal cannot follow ExploreDungeon's CancelWork (sim-actions-9).
                    if (cat != null && village.Routes.Any(r => r.CatId == cat.Id && RouteOutlivesCancel(village, r)))
                        return ActionResult.Fail(ResultCode.TransportReturning, "The cat must finish physically returning its transport first", (ResultArg.Target, cat.Id));
                    return HandleExploreDungeon(village, action, cat);
                case "recallexplorer":
                    return HandleRecallExplorer(village, cat);
                case "attackcreature":
                    return HandleAttackCreature(context, village, action, cat);
                default:
                    return ActionResult.Fail(ResultCode.UnknownAction, "Unknown action: " + action.Kind, (ResultArg.Kind, action.Kind ?? ""));
            }
        }

        /// <summary>
        /// True when cancelling this route would leave it physically returning (rail mid-edge, vessel away
        /// from its dock or loaded), so its driver stays bound to it. Mirrors CancelRoute without mutating.
        /// </summary>
        private bool RouteOutlivesCancel(Village v, TransportRoute route)
        {
            var vehicle = v.Vehicles.Find(x => x.Id == route.VehicleId);
            if (vehicle == null)
                return false;
            double x = vehicle.HasContinuousPosition ? vehicle.X : vehicle.Position.X, z = vehicle.HasContinuousPosition ? vehicle.Z : vehicle.Position.Z;
            bool accessibleWreck = !v.Cats.Any(c => c.Id == route.CatId && c.Alive) && Walkable(v, vehicle.Position);
            if (accessibleWreck)
                return false;
            if (route.Mode == "rail" && !AtTravelPoint(x, z, vehicle.Position))
                return true;
            return route.Mode == "shipping" && route.Path.Count > 0 && (!vehicle.Position.Equals(route.Path[0]) || !AtTravelPoint(x, z, route.Path[0]) || vehicle.Cargo.Count > 0 || vehicle.ItemIds.Count > 0);
        }

        private ActionResult HandleDeveloperClock(PlayerContext context, GameAction action, string kind)
        {
            if (!context.IsDeveloper)
                return ActionResult.Fail(ResultCode.DeveloperOnly, "Developer control disabled");
            if (kind == "advancetime")
                Step(Math.Clamp(action.Amount, 0, 86400));
            else if (kind == "settestrngseed")
                RandomState = unchecked((uint)action.Amount);
            else
                return ActionResult.Fail(ResultCode.NotImplemented, "Explicit AdvanceTime replaces implicit accelerated clocks");
            return ActionResult.Ok(ResultCode.Ok, "");
        }
        public static string Snake(string value) => string.Concat((value ?? "").Select((ch, i) => char.IsUpper(ch) ? (i > 0 ? "_" : "") + char.ToLowerInvariant(ch) : ch.ToString())).Replace(" ", "_");
    }
}
