using System;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class TradeSection : ISection
    {
        public string Id => "Trade";
        public string Label => "Trade";
        public string Group => "World";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var village = ctx.Village;
            var picks = ctx.Picks;
            UiKit.Text(root, "Trade uses finite goods. Village exchange needs returned-scout contact and a passable physical route.");
            picks.ResourceChoice(root);
            picks.AmountField(root, "Offer / buy amount");
            UiKit.Label(root, "Visiting trader · " + village.Trader.Phase, "subtitle");
            UiKit.Text(root, "Coins " + village.Coins.ToString("0") + " · trader stock " + UiText.Goods(village.Trader.Goods));
            UiKit.Button(root, "Buy resource", () => ctx.Act(new GameAction { Kind = "BuyResource", Resource = picks.SelectedResource, Amount = picks.Amount })).Named(UiNames.Act("buy", "resource"));
            var saleItems = village.Items.Where(i => village.Stockpiles.Any(p => p.Id == i.LocationId && p.Kind == "storage")).ToList();
            string saleItem = "";
            UiKit.Choice(root, "Stored item to sell", saleItems.Select(i => UiText.Pretty(i.Material) + " " + i.Kind + " · " + i.Id), "", x => saleItem = saleItems.Find(i => x.EndsWith(" · " + i.Id, StringComparison.Ordinal))?.Id ?? "").Named("choice-sale-item");
            UiKit.Button(root, "Sell goods", () => ctx.Act(new GameAction { Kind = "SellGoods", TargetId = saleItem })).Named(UiNames.Act("sell", "goods"));
            UiKit.Separator(root);
            var other = UiKit.Field(root, "Recipient village ID", picks.SelectedOther).Named("field-recipient-village");
            other.RegisterValueChangedCallback(e => picks.SelectedOther = e.newValue);
            UiKit.Text(root, "Known contacts: " + string.Join(", ", village.Contacts));
            string wanted = picks.SelectedResource == "water" ? "food" : "water";
            UiKit.Choice(root, "Request resource", Catalog.Resources.Where(r => r != "blessings"), wanted, x => wanted = x).Named("choice-request-resource");
            UiKit.Number(root, "Request amount", picks.OtherAmount, x => picks.OtherAmount = x).Named("field-request-amount");
            UiKit.Button(root, "Offer exchange", () => ctx.Act(new GameAction { Kind = "OfferVillageTrade", OtherVillageId = picks.SelectedOther, Resource = picks.SelectedResource, Amount = picks.Amount, OtherResource = wanted, OtherAmount = picks.OtherAmount })).Named(UiNames.Act("offer", "exchange"));
            foreach (var offer in ctx.World.TradeOffers.Where(t => t.FromVillageId == village.Id || t.ToVillageId == village.Id))
            {
                string id = offer.Id;
                var card = UiKit.Card(root, offer.Id + " · " + offer.Status);
                UiKit.Label(card, offer.Offered.Amount + " " + offer.Offered.Resource + " ⇄ " + offer.Requested.Amount + " " + offer.Requested.Resource, "body");
                UiKit.Button(card, "Accept", () => ctx.Act(new GameAction { Kind = "AcceptVillageTrade", TargetId = id })).Named(UiNames.Act("accept-trade", id));
                UiKit.Button(card, "Cancel / return escrow", () => ctx.Act(new GameAction { Kind = "CancelVillageTrade", TargetId = id })).Named(UiNames.Act("cancel-trade", id));
            }
            return root;
        }
    }
}
