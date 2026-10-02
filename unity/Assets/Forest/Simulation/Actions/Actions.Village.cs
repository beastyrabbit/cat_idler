using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleFoundVillage(PlayerContext context, Player player, GameAction action)
        {
            if (Villages.Any(v => v.OwnerId == context.PlayerId && !v.Communal))
                return ActionResult.Fail(ResultCode.OnePersonalVillage, "One personal village per identity");
            if (string.IsNullOrWhiteSpace(action.Name) || action.Name.Length > 48)
                return ActionResult.Fail(ResultCode.InvalidName, "Village name requires 1–48 characters", (ResultArg.Need, "48"), (ResultArg.Have, ActionResult.Number((action.Name ?? "").Length)));
            uint hash = Hash(context.PlayerId);
            var center = new Int2(80 + (int)(hash % 8) * 80, 80 + (int)((hash / 8) % 8) * 80);
            while (Villages.Any(v => Int2.Distance(v.Center, center) < 48) || !CanFoundAt("", center, false))
            {
                center.X += 80;
                if (center.X > 1000000)
                    return ActionResult.Fail(ResultCode.NoFoundingSite, "No unclaimed founding site available");
            }
            var village = Found(Id("village"), action.Name.Trim(), context.PlayerId, center, false);
            Villages.Add(village);
            EnsureVillageDungeon(village);
            player.PersonalVillageId = player.SelectedVillageId = village.Id;
            var result = ActionResult.Ok(ResultCode.Founded, village.Id);
            result.VillageId = village.Id;
            return result;
        }
        private ActionResult HandleJoinVillage(PlayerContext context, Player player, GameAction action)
        {
            string id = string.IsNullOrEmpty(action.TargetId) ? action.OtherVillageId : action.TargetId;
            var target = Village(id);
            if (!CanControl(context, target))
                return ActionResult.Fail(ResultCode.VillageDenied, "Village ownership denied", (ResultArg.Target, id ?? ""));
            player.SelectedVillageId = target.Id;
            // The wire contract keeps EntityId empty here; VillageId names the joined village.
            var result = ActionResult.Ok(ResultCode.Joined, "");
            result.VillageId = target.Id;
            return result;
        }
        private ActionResult HandleAssignOfficer(Village village, GameAction action, Cat cat)
        {
            int role = Array.IndexOf(Catalog.Roles, action.Role);
            if (role < 0)
                return ActionResult.Fail(ResultCode.UnknownRole, "Unknown role", (ResultArg.Role, action.Role ?? ""));
            if (cat == null)
                return ActionResult.Fail(ResultCode.CatRequired, "Living cat required", (ResultArg.Target, action.CatId ?? ""));
            if (!village.Research.Contains(Catalog.RoleStudies[role]) || !village.Buildings.Any(b => b.Kind == Catalog.RoleBuildings[role] && b.Completed))
                return ActionResult.Fail(ResultCode.OfficeLocked, "Office requires completed " + Catalog.RoleBuildings[role] + " and " + Catalog.RoleStudies[role], (ResultArg.Role, action.Role), (ResultArg.Building, Catalog.RoleBuildings[role]), (ResultArg.Study, Catalog.RoleStudies[role]));
            var held = village.Officers.Find(o => o.CatId == cat.Id && o.Role != action.Role);
            if (held != null)
                return ActionResult.Fail(ResultCode.AlreadyOfficer, "Cat already holds an office", (ResultArg.Target, cat.Id), (ResultArg.Role, held.Role));
            foreach (var replaced in village.Officers.Where(o => o.Role == action.Role))
            {
                var previous = Cat(replaced.CatId);
                if (previous != null)
                    previous.OfficerRole = "";
            }
            village.Officers.RemoveAll(o => o.Role == action.Role);
            village.Officers.Add(new Officer { Role = action.Role, CatId = cat.Id });
            cat.OfficerRole = action.Role;
            return ActionResult.Ok(ResultCode.Appointed, cat.Id, (ResultArg.Role, action.Role));
        }
        private ActionResult HandleUnassignOfficer(Village village, GameAction action)
        {
            var holder = village.Officers.Find(o => o.Role == action.Role);
            if (holder == null)
                return ActionResult.Fail(ResultCode.OfficeVacant, "Office already vacant", (ResultArg.Role, action.Role ?? ""));
            var h = Cat(holder.CatId);
            if (h != null)
                h.OfficerRole = "";
            village.Officers.Remove(holder);
            StopOfficerWork(village, action.Role);
            return ActionResult.Ok(ResultCode.Dismissed, holder.CatId, (ResultArg.Role, action.Role));
        }
        private ActionResult HandleCastVote(PlayerContext context, Village village, Cat cat)
        {
            if (village.Election == null)
                return ActionResult.Fail(ResultCode.NoElection, "Open election required");
            if (cat == null)
                return ActionResult.Fail(ResultCode.CatRequired, "Living candidate required");
            village.Election.Votes.RemoveAll(b => b.PlayerId == context.PlayerId);
            village.Election.Votes.Add(new Ballot { PlayerId = context.PlayerId, CatId = cat.Id });
            return ActionResult.Ok(ResultCode.Voted, cat.Id, (ResultArg.Target, village.Election.Id));
        }
        private ActionResult HandleRequestVoteKick(PlayerContext context, Village village)
        {
            if (!village.Cats.Any(c => c.Id == village.LeaderId && c.Alive))
                return ActionResult.Fail(ResultCode.LeaderRequired, "Living leader required");
            if (village.KickPetition == null)
                village.KickPetition = new Election { Id = Id("petition"), Kind = "vote_kick", TargetId = village.LeaderId, EndAt = TimeSeconds + 600 };
            if (village.KickPetition.EndAt <= TimeSeconds)
                return ActionResult.Fail(ResultCode.PetitionClosed, "Petition closed", (ResultArg.Target, village.KickPetition.Id));
            if (!village.KickPetition.Votes.Any(b => b.PlayerId == context.PlayerId))
                village.KickPetition.Votes.Add(new Ballot { PlayerId = context.PlayerId, CatId = village.KickPetition.TargetId });
            int signatures = village.KickPetition.Votes.Select(b => b.PlayerId).Distinct().Count();
            return ActionResult.Ok(ResultCode.Signed, village.KickPetition.Id, (ResultArg.Have, ActionResult.Number(signatures)), (ResultArg.Need, ActionResult.Number(PetitionSignatures)));
        }
    }
}
