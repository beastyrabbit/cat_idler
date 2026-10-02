using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace IdleCatForest.Tests
{
    /// <summary>
    /// Drives the HUD the way a player does, through stable element names (UiNames) and the internal
    /// HUD API, never through label text or private members.
    /// </summary>
    public static class UiTestKit
    {
        public static VisualElement Root(ForestGame game) => game.GetComponent<UIDocument>().rootVisualElement;

        /// <summary>The only element of type <typeparamref name="T"/> with this name.</summary>
        public static T Find<T>(ForestGame game, string name) where T : VisualElement
        {
            var matches = Root(game).Query<T>(name).ToList();
            Assert.That(matches, Has.Count.EqualTo(1), "Expected exactly one " + typeof(T).Name + " named '" + name + "'.");
            return matches[0];
        }

        /// <summary>The first element of type <typeparamref name="T"/> with this name, or null.</summary>
        public static T FindFirst<T>(ForestGame game, string name) where T : VisualElement => Root(game).Q<T>(name);

        /// <summary>Every element of type <typeparamref name="T"/> whose name starts with <paramref name="prefix"/>.</summary>
        public static List<T> WithPrefix<T>(ForestGame game, string prefix) where T : VisualElement =>
            Root(game).Query<T>().Where(e => e.name != null && e.name.StartsWith(prefix, StringComparison.Ordinal)).ToList();

        public static Button Button(ForestGame game, string name) => Find<Button>(game, name);

        /// <summary>Submits the only button with this name, as a click or Enter would.</summary>
        public static void Click(ForestGame game, string name) => Click(Button(game, name));

        public static void Click(Button button)
        {
            Assert.That(button, Is.Not.Null);
            using (var submit = NavigationSubmitEvent.GetPooled())
            {
                submit.target = button;
                button.SendEvent(submit);
            }
        }

        /// <summary>One HUD frame without the 0.5 s refresh: live values, tethered card, state.</summary>
        public static void Tick(ForestGame game) => game.UI.Tick();

        /// <summary>The 0.5 s HUD refresh: chrome text and the open section's Refresh.</summary>
        public static void Refresh(ForestGame game) => game.UI.RefreshHud();

        /// <summary>A world tile click while a placement is active.</summary>
        public static void PlaceAt(ForestGame game, Int2 tile) => game.UI.Placement.Click(tile);
    }
}
