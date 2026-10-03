using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// What a button says for two seconds after its action succeeded. A confirmation reuses the verb
    /// ("Planned", "Den planned", "Studied Basic Tools"); routine steering confirms with nothing.
    /// </summary>
    public static class Confirmations
    {
        /// <summary>Verb per action kind (lowercase, underscores removed, as the router matches them).</summary>
        private static readonly Dictionary<string, string> Verbs = new Dictionary<string, string>
        {
            ["acceptvillagetrade"] = "Accepted",
            ["assignofficer"] = "Appointed",
            ["assignworker"] = "Assigned",
            ["attackcreature"] = "Attacking",
            ["boost"] = "Sped up",
            ["boostcat"] = "Cheered on",
            ["buildbridge"] = "Bridge planned",
            ["builddock"] = "Dock planned",
            ["buildroad"] = "Road planned",
            ["buildtransportvehicle"] = "Ordered",
            ["buyresource"] = "Bought",
            ["canceltransportroute"] = "Route cancelled",
            ["cancelvillagetrade"] = "Offer withdrawn",
            ["castvote"] = "Voted",
            ["clearfarm"] = "Field cleared",
            ["createtransportroute"] = "Route set",
            ["createzone"] = "Zone marked",
            ["defendraid"] = "Rallied",
            ["designatefarm"] = "Field planned",
            ["designatefishingspot"] = "Fishing spot marked",
            ["designategatherspot"] = "Gathering spot marked",
            ["designaterail"] = "Track planned",
            ["designatestockpile"] = "Stockpile marked",
            ["dispatchscout"] = "Scout sent",
            ["editproductionqueue"] = "Queue updated",
            ["editproductionworkslot"] = "Queue updated",
            ["entercat"] = "",
            ["entercatcontrol"] = "",
            ["equipitem"] = "Equipped",
            ["exploredungeon"] = "Set out",
            ["haulgatherspot"] = "Hauling",
            ["interactcat"] = "",
            ["keepcatcontrol"] = "",
            ["leavecat"] = "",
            ["leavecatcontrol"] = "",
            ["movecat"] = "",
            ["offermaterials"] = "Offered",
            ["offerresource"] = "Offered",
            ["offertithe"] = "Offered",
            ["offervillagetrade"] = "Offer sent",
            ["planbuilding"] = "Planned",
            ["purchaseupgrade"] = "Blessed",
            ["recallexplorer"] = "Called home",
            ["removegatherspot"] = "Removed",
            ["removestockpile"] = "Removed",
            ["removezone"] = "Removed",
            ["repairitem"] = "Sent for repair",
            ["requestjob"] = "Sent",
            ["requestvotekick"] = "Petition signed",
            ["researchnode"] = "Studied",
            ["sellgoods"] = "Sold",
            ["setcatlaborpreference"] = "Updated",
            ["settestacceleration"] = "",
            ["settestrngseed"] = "",
            ["trainwarrior"] = "Training",
            ["unassignofficer"] = "Office vacated",
            ["unequipitem"] = "Unequipped",
            ["unlocknode"] = "Studied",
            // Actions W2-06 adds (names from its brief).
            ["releaseworker"] = "Released from work",
            ["stopcat"] = "",
            ["hurrystudy"] = "Hurried",
            // Kinds wired into the router in Wave 2.
            ["preferforwork"] = "Preference saved",
            ["cancelconstruction"] = "Site cancelled",
            ["demolish"] = "Taken down",
            ["demolishbuilding"] = "Taken down",
            ["foundvillage"] = "Colony founded",
            ["joinvillage"] = "Joined",
            ["ensure"] = "",
            ["presence"] = "",
            ["advancetime"] = "",
        };

        /// <summary>The short verb for an action kind: "Planned". "" means show nothing.</summary>
        public static string Confirm(string actionKind)
        {
            string key = Key(actionKind);
            if (Verbs.TryGetValue(key, out var verb))
                return verb;
            return key == "" ? "" : "Done";
        }

        /// <summary>True when the action kind has an authored confirmation (Coverage).</summary>
        public static bool IsAuthored(string actionKind) => Verbs.ContainsKey(Key(actionKind));

        /// <summary>
        /// The confirmation with its subject: "Den planned", "Studied Basic Tools", "Hazel is yours · Tab
        /// to hand back". Steering a cat confirms with "".
        /// </summary>
        public static string Confirm(GameAction action, Village village = null)
        {
            if (action == null)
                return "";
            string key = Key(action.Kind);
            var found = village?.Cats.Find(c => c.Id == action.CatId);
            string cat = found == null || string.IsNullOrEmpty(found.Name) ? "" : found.Name;
            switch (key)
            {
                case "planbuilding":
                    return action.Name == "" ? "Planned" : Names.Building(action.Name) + " planned";
                case "researchnode":
                case "unlocknode":
                    return action.NodeId == "" ? "Studied" : "Studied " + Names.Study(action.NodeId);
                case "entercat":
                case "entercatcontrol":
                    return (cat == "" ? "You're steering" : cat + " is yours") + " · Tab to hand back";
                case "assignworker":
                    return cat == "" ? "Assigned" : cat + " assigned";
                case "assignofficer":
                    if (action.Role == "")
                        return "Appointed";
                    return cat == "" ? Names.Role(action.Role) + " appointed" : cat + " is now " + Names.Role(action.Role);
                case "designatefarm":
                    return Names.Of(NameKind.Crop, action.Resource == "" ? "grain" : action.Resource) + " field planned";
                case "purchaseupgrade":
                    return action.Name == "" ? "Blessed" : Names.Of(NameKind.Upgrade, action.Name) + " improved";
                case "requestjob":
                    return action.Labor == "" ? "Sent" : Names.Of(NameKind.Labor, action.Labor) + " started";
                case "offerresource":
                    return action.Resource == "" ? "Offered" : "Offered " + Names.Amount(action.Amount, action.Resource);
                default:
                    return Confirm(action.Kind);
            }
        }

        /// <summary>The router's normalisation: lowercase without underscores.</summary>
        public static string Key(string actionKind) =>
            string.IsNullOrEmpty(actionKind) ? "" : actionKind.Replace("_", "").ToLowerInvariant();
    }
}
