using System.Linq;
using IdleCatForest.Simulation;
using NUnit.Framework;

public class SimulationTests
{
    [Test] public void FreshFoundingHasExactFiniteBedsAndSupplies()
    {
        var w=World.Create(7); var v=w.Villages.Single();
        Assert.That(v.Cats.Count(c=>c.Alive),Is.EqualTo(30));
        Assert.That(v.Buildings.Count(b=>b.Kind=="den"&&b.Completed),Is.EqualTo(6));
        Assert.That(w.Total(v,"food"),Is.EqualTo(100));
        Assert.That(w.Total(v,"water"),Is.EqualTo(200));
        Assert.That(w.Total(v,"thread"),Is.Zero);
    }
    [Test] public void CompleteMaintainedCatalogHasNoMissingEntitlements()
    {
        Assert.That(Catalog.Recipes.Count,Is.EqualTo(108));
        Assert.That(Catalog.Research.Count,Is.EqualTo(487));
        foreach(var r in Catalog.Recipes) Assert.That(r.Founding||Catalog.Research.Any(n=>n.Payloads.Any(p=>p.Kind=="unlock_recipe"&&p.Id==r.Id)),Is.True,r.Id);
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
