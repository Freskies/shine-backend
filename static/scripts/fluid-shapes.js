/*
 * Pauses the blob clip-path animation on the shapes that are off-screen.
 *
 * The animation is main-thread work that no compositor can take over: each update changes
 * the clip geometry, so the element is re-rasterized and re-clipped against a 64-vertex
 * polygon. The home page carries 28 `.fluid-shape` elements and shows perhaps eight of them
 * at a time, so most of that cost buys nothing.
 *
 * Progressive enhancement in the honest direction: the paused class is only ever *added*
 * here. With this script blocked, every shape animates exactly as it did before.
 */

(() => {
	const body = document.querySelector("body");
	const pausedClass = "fluid-shape--paused";

	/*
	 * Under `prefers-reduced-motion` the CSS already drops the animation to a fixed pose, so
	 * there is nothing to pause and no need to restate the query's logic in the second place.
	 */
	if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
	if (!("IntersectionObserver" in window)) return;

	const observer = new IntersectionObserver(
		entries => {
			for (const entry of entries) {
				entry.target.classList.toggle(pausedClass, !entry.isIntersecting);
			}
		},
		/*
		 * A fifth of a viewport of lead-in is enough. The margin is not there to hide a
		 * restart — pausing keeps the angle, so resuming is seamless whenever it happens —
		 * only to let the browser rasterize the first frame before the shape is on screen.
		 */
		{ rootMargin: "20% 0px" },
	);

	const observed = new WeakSet();

	function observeShapesIn (root) {
		for (const shape of root.querySelectorAll(".fluid-shape")) {
			if (observed.has(shape)) continue;
			observed.add(shape);
			observer.observe(shape);
		}
	}

	observeShapesIn(document);

	/*
	 * htmx swaps in markup carrying shapes of its own (the enrolment confirmation), and those
	 * nodes are not in the document when the scan above runs.
	 */
	body.addEventListener("htmx:afterSwap", event => {
		if (event.target instanceof Element) observeShapesIn(event.target);
	});
})();
