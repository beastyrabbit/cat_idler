using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Store and feedback links. There is no Steam App ID yet (D10): the store URL is null and the
    /// wishlist reads "coming soon". W3-06 fills these in.
    /// </summary>
    public static class ShellLinks
    {
        public static string StoreUrl => null;

        public static bool WishlistAvailable => !string.IsNullOrEmpty(StoreUrl);

        public static string FeedbackUrl => null;

        /// <summary>Opens the store page when one exists; returns whether it did.</summary>
        public static bool OpenStore()
        {
            if (!WishlistAvailable) return false;
            Application.OpenURL(StoreUrl);
            return true;
        }
    }
}
