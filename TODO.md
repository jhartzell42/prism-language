* Combinators
    * Basics for dynamic
        * `map`, `map2` `map3`
* Widgets
    * `Widget<T>` has a callback
        * Returns a `WidgetNode` and a `T`
        * Event fires on completion
            * Containing `Arc<WidgetNode>`
    * This allows creation of cyclic events/behaviors
        * How to manage infinite regressions?
            * Good error messages
            * Recovery? Should widget creation fail sometimes?
                * What does this mean?
    * `WidgetNode` is:
        * Indexed events
        * Indexed dynamics
        * Child `WidgetNode`s
        * Optional string tags (variable names!)
    * There is a root widget node
    * Functions returning `Widget<T>` can be used with:
        * `dyn` combinator
        * `list` combinator
    * Figure out how external-world interaction should work
        * Make sure it's completely dynamic
            * Stubbable for testing purposes
            * Able to run with multiple backends
            * Inspectable to attached debuggers
            * Users of Prism Language should be able to access this
                * To provide in-language impls based on lower-level primitives
                * To run test cases
        * Make sure it can involve containers
            * Widgets align themselves in hierarchies in GUIs
        * Idea #1:
            * Load up backend with a bunch of widgets
                * Backend can be combination of multiple compatible bundles
            * `WidgetNode` provides hook information to backend
                * Triggers
                * Widget selectors (default is just blank)
                * Widget configuration in the form of `serde_json::Value`
                * Can query backend for support?
            * Backend gets notified whenever widget map changes
                * I guess by event, why not?
                * Wires itself up accordingly
                * Backend runs main loop
* Implement programming language
    * v1: As translator to generate a Rust module
    * v2: As interpreter