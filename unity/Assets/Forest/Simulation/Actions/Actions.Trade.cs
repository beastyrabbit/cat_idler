

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleBuildTransportVehicle(Village village, GameAction action, Cat cat) => CreateInfrastructure(village, action, cat, action.Mode == "shipping" ? "vessel" : "wagon", action.Mode == "shipping" ? "lumber" : "metal", 8);
        private ActionResult HandleCreateTransportRoute(Village village, GameAction action, Cat cat) => CreateRoute(village, action, cat);
        private ActionResult HandleCancelTransportRoute(Village village, GameAction action) => CancelRoute(village, action.TargetId);
        private ActionResult HandleOfferVillageTrade(Village village, GameAction action) => OfferTrade(village, action);
        private ActionResult HandleAcceptVillageTrade(Village village, GameAction action) => AcceptTrade(village, action.TargetId);
        private ActionResult HandleCancelVillageTrade(Village village, GameAction action) => CancelTrade(village, action.TargetId);
        private ActionResult HandleBuyResource(Village village, GameAction action) => Buy(village, action);
        private ActionResult HandleSellGoods(Village village, GameAction action) => Sell(village, action);
    }
}
