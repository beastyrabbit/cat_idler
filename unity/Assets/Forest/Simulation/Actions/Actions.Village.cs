using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleFoundVillage(PlayerContext context, Player player, GameAction action)
        {
            if (Villages.Any(v => v.OwnerId == context.PlayerId && !v.Communal))
                return ActionResult.Fail("One personal village per identity");
            if (string.IsNullOrWhiteSpace(action.Name) || action.Name.Length > 48)
                return ActionResult.Fail("Village name requires 1–48 characters");
            uint hash = Hash(context.PlayerId);
            var center = new Int2(80 + (int)(hash % 8) * 80, 80 + (int)((hash / 8) % 8) * 80);
            while (Villages.Any(v => Int2.Distance(v.Center, center) < 48) || !CanFoundAt("", center, false))
            {
                center.X += 80;
                if (center.X > 1000000)
                    return ActionResult.Fail("No unclaimed founding site available");
            }
            var village = Found(Id("village"), action.Name.Trim(), context.PlayerId, center, false);
            Villages.Add(village);
            EnsureVillageDungeon(village);
            player.PersonalVillageId = player.SelectedVillageId = village.Id;
            return new ActionResult { Success = true, EntityId = village.Id, VillageId = village.Id };
        }
        private ActionResult HandleJoinVillage(PlayerContext context, Player player, GameAction action)
        {
            var target = Village(string.IsNullOrEmpty(action.TargetId) ? action.OtherVillageId : action.TargetId);
            if (!CanControl(context, target))
                return ActionResult.Fail("Village ownership denied");
            player.SelectedVillageId = target.Id;
            return new ActionResult { Success = true, VillageId = target.Id };
        }
        private ActionResult HandleAssignOfficer(Village village, GameAction action, Cat cat)
        {
            int role = Array.IndexOf(Catalog.Roles, action.Role);
            if (role < 0 || cat == null)
                return ActionResult.Fail("Unknown role or living cat");
            if (!village.Research.Contains(Catalog.RoleStudies[role]) || !village.Buildings.Any(b => b.Kind == Catalog.RoleBuildings[role] && b.Completed))
                return ActionResult.Fail("Office requires completed " + Catalog.RoleBuildings[role] + " and " + Catalog.RoleStudies[role]);
            if (village.Officers.Any(o => o.CatId == cat.Id && o.Role != action.Role))
                return ActionResult.Fail("Cat already holds an office");
            foreach (var replaced in village.Officers.Where(o => o.Role == action.Role))
            {
                var previous = Cat(replaced.CatId);
                if (previous != null)
                    previous.OfficerRole = "";
            }
            village.Officers.RemoveAll(o => o.Role == action.Role);
            village.Officers.Add(new Officer { Role = action.Role, CatId = cat.Id });
            cat.OfficerRole = action.Role;
            return ActionResult.Ok();
        }
        private ActionResult HandleUnassignOfficer(Village village, GameAction action)
        {
            var holder = village.Officers.Find(o => o.Role == action.Role);
            if (holder == null)
                return ActionResult.Fail("Office already vacant");
            var h = Cat(holder.CatId);
            if (h != null)
                h.OfficerRole = "";
            village.Officers.Remove(holder);
            StopOfficerWork(village, action.Role);
            return ActionResult.Ok();
        }
        private ActionResult HandleCastVote(PlayerContext context, Village village, Cat cat)
        {
            if (village.Election == null || cat == null)
                return ActionResult.Fail("Open election and living candidate required");
            village.Election.Votes.RemoveAll(b => b.PlayerId == context.PlayerId);
            village.Election.Votes.Add(new Ballot { PlayerId = context.PlayerId, CatId = cat.Id });
            return ActionResult.Ok();
        }
        private ActionResult HandleRequestVoteKick(PlayerContext context, Village village)
        {
            if (!village.Cats.Any(c => c.Id == village.LeaderId && c.Alive))
                return ActionResult.Fail("Living leader required");
            if (village.KickPetition == null)
                village.KickPetition = new Election { Id = Id("petition"), Kind = "vote_kick", TargetId = village.LeaderId, EndAt = TimeSeconds + 600 };
            if (village.KickPetition.EndAt <= TimeSeconds)
                return ActionResult.Fail("Petition closed");
            if (!village.KickPetition.Votes.Any(b => b.PlayerId == context.PlayerId))
                village.KickPetition.Votes.Add(new Ballot { PlayerId = context.PlayerId, CatId = village.KickPetition.TargetId });
            return ActionResult.Ok(village.KickPetition.Id);
        }
    }
}
