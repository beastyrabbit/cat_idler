using System;
using IdleCatForest.Simulation;
class Smoke
{
    static void Require(bool ok,string message){if(!ok)throw new Exception(message);}
    static void Main()
    {
        var w=World.Create(7);var v=w.Villages[0];
        Require(v.Cats.Count==30,"Founding population");Require(Catalog.Recipes.Count==108&&Catalog.Research.Count==487,"Catalog missing");
        v.Stockpiles[0].Goods.Clear();v.Stockpiles[0].Goods.Add(new Stack("logs",5));
        Require(w.Reserve(v,"a","logs",5),"First claim");Require(!w.Reserve(v,"b","logs",1),"Double claim");w.Release("a");Require(w.Reserve(v,"b","logs",5),"Claim release");
        var c=new PlayerContext{PlayerId="alice"};var made=w.Apply(c,new GameAction{Kind="FoundVillage",Name="Alice"});Require(made.Success,"Found personal");
        var personal=w.Village(made.EntityId);Require(!w.Apply(new PlayerContext{PlayerId="bob",VillageId=personal.Id},new GameAction{Kind="EnterCatControl",CatId=personal.Cats[0].Id}).Success,"Foreign control");
        personal.Items.Add(new Item{Id="finite-tool",Kind="tool",Material="wood",VillageId=personal.Id,LocationId=personal.Stockpiles[0].Id});
        Require(w.Total(personal,"tools")==1,"Finite equipment must project exactly once");
        var carrier=personal.Cats[0];var job=new Job{Id="blocked",Kind="logs",Phase="output_delivery",CatId=carrier.Id,Position=carrier.Position};personal.Jobs.Add(job);carrier.JobId=job.Id;carrier.Cargo.Add(new Stack("logs",8));carrier.Thirst=1;
        w.Step(1);Require(carrier.Goal=="need_drink","Blocked cargo must not prevent critical drinking");
        var upgrades=World.Create(12);var uv=upgrades.Villages[0];uv.Blessings=100;
        Require(upgrades.Apply(new PlayerContext{PlayerId="test"},new GameAction{Kind="PurchaseUpgrade",Name="clickPower"}).Success,"Maintained click-power upgrade must be purchasable");
        Require(uv.Blessings==98&&World.Amount(uv.UpgradeLevels,"click_power")==1,"Upgrade cost and persistent level");
        var pending=World.Create(14);var pv=pending.Villages[0];var builder=pv.Cats[0];var planned=new Job{Id="imported-build",Kind="build",PendingBuildingKind="workshop",CatId=builder.Id,Progress=3};pv.Jobs.Add(planned);builder.JobId=planned.Id;
        pending.Step(1);Require(!planned.Completed&&planned.TargetId!=""&&pv.Buildings.Exists(b=>b.Id==planned.TargetId),"Queued imported build must resolve a real scaffold without discarding work");
        Require(pv.Radius==9,"Communal founding must preserve maintained larger parcel");
        Require(pv.Buildings.FindAll(b=>b.Completed).Count==16,"Communal founding retains completed production yards and service buildings");
        var routes=World.Create(7);var rv=routes.Villages[0];Require(routes.Path(new Int2(0,-14),rv.Center,rv)!=null,"A worker on the known north route must have a reciprocal route home");
        var access=World.Create(41);var founded=access.Apply(new PlayerContext{PlayerId="acceptance-player"},new GameAction{Kind="FoundVillage",Name="Supply"});var av=access.Village(founded.EntityId);
        Require(access.Apply(new PlayerContext{PlayerId="acceptance-player",VillageId=av.Id},new GameAction{Kind="RequestJob",Name="hunt"}).Success,"Every personal founding has reachable food");
        Require(access.Apply(new PlayerContext{PlayerId="acceptance-player",VillageId=av.Id},new GameAction{Kind="RequestJob",Name="water"}).Success,"Every personal founding has reachable shoreline water");
        Console.WriteLine("Founding, complete catalog, reservation, authority and finite equipment assertions passed");
    }
}
