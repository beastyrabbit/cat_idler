using System.Linq;
using IdleCatForest.Simulation;
using NUnit.Framework;

public class SimulationTests
{
    // Invariants rather than counts: founding numbers and the authored catalog may be retuned.
    [Test] public void FreshFoundingHousesEveryCatWithFiniteSupplies()
    {
        var w=World.Create(7); var v=w.Villages.Single();
        var living=v.Cats.Where(c=>c.Alive).ToList();
        var dens=v.Buildings.Where(b=>b.Kind=="den"&&b.Completed).ToList();
        Assert.That(living,Is.Not.Empty);
        Assert.That(dens.Count*w.DenCapacity(v),Is.GreaterThanOrEqualTo(living.Count),"beds for every founding cat");
        foreach(var c in living) Assert.That(dens.Any(d=>d.Id==c.BedId),Is.True,c.Name+" has no founding bed");
        foreach(var den in dens) Assert.That(living.Count(c=>c.BedId==den.Id),Is.LessThanOrEqualTo(w.DenCapacity(v)),den.Id+" is overbooked");
        Assert.That(w.Total(v,"food"),Is.GreaterThan(0));
        Assert.That(w.Total(v,"water"),Is.GreaterThan(0));
        Assert.That(World.Finite(w.Total(v,"food"))&&World.Finite(w.Total(v,"water")),Is.True);
        Assert.That(w.Total(v,"thread"),Is.Zero);
    }
    [Test] public void AuthoredCatalogHasNoMissingEntitlements()
    {
        Assert.That(Catalog.Recipes,Is.Not.Empty);
        Assert.That(Catalog.Research,Is.Not.Empty);
        Assert.That(Catalog.Recipes.Select(r=>r.Id).Distinct().Count(),Is.EqualTo(Catalog.Recipes.Count),"recipe ids are unique");
        Assert.That(Catalog.Research.Select(n=>n.Id).Distinct().Count(),Is.EqualTo(Catalog.Research.Count),"study ids are unique");
        foreach(var r in Catalog.Recipes)
        {
            Assert.That(r.Founding||Catalog.Research.Any(n=>n.Payloads.Any(p=>p.Kind=="unlock_recipe"&&p.Id==r.Id)),Is.True,r.Id+" has no unlock");
            Assert.That(Catalog.Buildings.Contains(r.Building),Is.True,r.Id+" names an unknown station");
            Assert.That(r.Seconds,Is.GreaterThan(0),r.Id);
            Assert.That(r.Inputs.All(s=>s.Amount>0)&&r.Outputs.All(s=>s.Amount>0),Is.True,r.Id+" has an empty stack");
            Assert.That(r.Outputs.Count>0||r.ItemKind!="",Is.True,r.Id+" makes nothing");
        }
        foreach(var n in Catalog.Research)
        {
            Assert.That(n.Cost,Is.GreaterThan(0),n.Id);
            foreach(var p in n.Prerequisites) Assert.That(Catalog.Study(p),Is.Not.Null,n.Id+" requires unknown "+p);
            foreach(var p in n.Payloads.Where(p=>p.Kind=="unlock_recipe")) Assert.That(Catalog.Recipe(p.Id),Is.Not.Null,n.Id+" unlocks unknown recipe "+p.Id);
        }
        // Acyclic: repeatedly owning every study whose prerequisites are owned reaches the whole tree.
        var owned=new System.Collections.Generic.HashSet<string>();
        bool progress=true;
        while(progress){progress=false;foreach(var n in Catalog.Research.Where(n=>!owned.Contains(n.Id)&&n.Prerequisites.All(owned.Contains))){owned.Add(n.Id);progress=true;}}
        Assert.That(owned.Count,Is.EqualTo(Catalog.Research.Count),"cyclic or unreachable studies: "+string.Join(",",Catalog.Research.Where(n=>!owned.Contains(n.Id)).Select(n=>n.Id)));
    }
    [Test] public void ForeignVillageAndDirectControlAreRejected()
    {
        var w=World.Create(7); var a=new PlayerContext{PlayerId="alice"};
        var made=w.Apply(a,new GameAction{Kind="FoundVillage",Name="Alice"}); Assert.That(made.Success,Is.True);
        var v=w.Villages.Single(x=>x.OwnerId=="alice");
        var b=new PlayerContext{PlayerId="bob",VillageId=v.Id};
        Assert.That(w.Apply(b,new GameAction{Kind="EnterCat",CatId=v.Cats[0].Id}).Success,Is.False);
        Assert.That(w.Apply(b,new GameAction{Kind="RequestJob",Name="hunt"}).Success,Is.False);
    }
    [Test] public void ScarceGoodsCannotBeClaimedTwice()
    {
        var w=World.Create(9); var v=w.Villages[0]; var pile=v.Stockpiles[0];
        pile.Goods.Clear(); pile.Goods.Add(new Stack("logs",5));
        Assert.That(w.Reserve(v,"job-a","logs",5),Is.True);
        Assert.That(w.Reserve(v,"job-b","logs",1),Is.False);
        Assert.That(w.Total(v,"logs"),Is.EqualTo(5));
        w.Release("job-a"); Assert.That(w.Reserve(v,"job-b","logs",5),Is.True);
    }
    [Test] public void BlockedCarrierPreservesGoodsWhileMeetingCriticalThirst()
    {
        var w=World.Create(7);var v=w.Villages[0];var c=v.Cats[0];var j=new Job{Id="blocked-haul",Kind="logs",Phase="output_delivery",CatId=c.Id,Position=c.Position};
        v.Jobs.Add(j);c.JobId=j.Id;c.Cargo.Add(new Stack("logs",8));c.Thirst=1;v.Stockpiles[0].Capacity=v.Stockpiles[0].Goods.Sum(s=>s.Amount);
        w.Step(1);
        Assert.That(c.Goal,Is.EqualTo("need_drink"));
        Assert.That(v.Stockpiles.Where(s=>s.Kind=="spill").Sum(s=>World.Amount(s.Goods,"logs")),Is.EqualTo(8));
        Assert.That(j.Completed,Is.False);
    }
    [Test] public void BusyCatCannotOwnAnotherConstructionOrHaul()
    {
        var w=World.Create(7);var v=w.Villages[0];var c=v.Cats[0];c.JobId="existing";v.Jobs.Add(new Job{Id="existing",Kind="scout",CatId=c.Id});
        var action=new GameAction{Kind="PlanBuilding",Name="research_hut",Position=new Int2(-5,2),CatId=c.Id};
        Assert.That(w.Apply(new PlayerContext{PlayerId="a"},action).Success,Is.False);
        Assert.That(c.JobId,Is.EqualTo("existing"));Assert.That(w.Validate(),Is.Empty);
    }
}
