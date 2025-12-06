use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SearchBarProps<T: Clone + PartialEq + 'static> {
    /// Items to search through
    pub items: Vec<T>,
    /// Function to filter items based on search term
    pub filter_fn: fn(&T, &str) -> bool,
    /// Function to render an item in the dropdown
    pub render_item: fn(&T) -> Element,
    /// Function to get a unique key for an item
    pub get_key: fn(&T) -> String,
    /// Callback when an item is selected
    pub on_select: EventHandler<T>,
    /// Placeholder text for the search input
    #[props(default = "Search...".to_string())]
    pub placeholder: String,
    /// Maximum number of items to show in dropdown
    #[props(default = 30)]
    pub max_results: usize,
    /// Whether the search bar is disabled
    #[props(default = false)]
    pub disabled: bool,
}

#[component]
pub fn SearchBar<T: Clone + PartialEq + 'static>(props: SearchBarProps<T>) -> Element {
    let mut search_term = use_signal(String::new);
    let mut show_dropdown = use_signal(|| false);
    let mut highlighted_index = use_signal(|| None::<usize>);

    // Track props in a signal so the memo can react to changes
    let mut items_signal = use_signal(|| props.items.clone());
    if *items_signal.peek() != props.items {
        items_signal.set(props.items.clone());
    }
    let filter_fn = props.filter_fn;
    let max_results = props.max_results;

    let filtered_items = use_memo(move || {
        let term = search_term.read().to_lowercase();
        if term.is_empty() {
            return vec![];
        }

        items_signal
            .read()
            .iter()
            .filter(|item| (filter_fn)(item, &term))
            .take(max_results)
            .cloned()
            .collect::<Vec<_>>()
    });

    // Update highlight when filtered items change
    // Use peek() to read highlighted_index without subscribing to avoid re-triggering
    use_effect(move || {
        let current_filtered = filtered_items.read();
        let should_show = *show_dropdown.read();
        let current_highlight = *highlighted_index.peek();

        if should_show && !current_filtered.is_empty() {
            if current_highlight.is_none_or(|idx| idx >= current_filtered.len()) {
                highlighted_index.set(Some(0));
            }
        } else if current_highlight.is_some() {
            highlighted_index.set(None);
        }
    });

    let scroll_to_item = |index: usize| {
        let element_id = format!("search-item-{index}");
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;
            #[wasm_bindgen(inline_js = r#"
                export function scroll_to_element(id) {
                    const element = document.getElementById(id);
                    if (element) {
                        element.scrollIntoView({ block: 'nearest', inline: 'nearest' });
                    }
                }
            "#)]
            extern "C" {
                fn scroll_to_element(id: &str);
            }
            scroll_to_element(&element_id);
        }
    };

    let mut handle_select = move |item: T| {
        props.on_select.call(item);
        search_term.set(String::new());
        show_dropdown.set(false);
        highlighted_index.set(None);
    };

    let handle_keyboard = move |evt: KeyboardEvent| {
        let current_filtered = filtered_items.read();

        match evt.key() {
            Key::ArrowDown => {
                if current_filtered.is_empty() {
                    return;
                }
                evt.prevent_default();

                let next_idx = match *highlighted_index.read() {
                    Some(idx) => (idx + 1) % current_filtered.len(),
                    None => 0,
                };
                highlighted_index.set(Some(next_idx));
                scroll_to_item(next_idx);
            }
            Key::ArrowUp => {
                if current_filtered.is_empty() {
                    return;
                }
                evt.prevent_default();

                let next_idx = match *highlighted_index.read() {
                    Some(0) => current_filtered.len() - 1,
                    Some(idx) => idx - 1,
                    None => current_filtered.len() - 1,
                };
                highlighted_index.set(Some(next_idx));
                scroll_to_item(next_idx);
            }
            Key::Enter => {
                evt.prevent_default();
                if let Some(idx) = *highlighted_index.read() {
                    if let Some(item) = current_filtered.get(idx) {
                        handle_select(item.clone());
                    }
                }
            }
            Key::Escape => {
                show_dropdown.set(false);
                highlighted_index.set(None);
            }
            _ => {}
        }
    };

    rsx! {
        div { class: "relative w-full",
            input {
                "type": "text",
                class: "input w-full h-10",
                placeholder: "{props.placeholder}",
                value: "{search_term}",
                autocomplete: "off",
                disabled: props.disabled,
                oninput: move |evt| {
                    let new_value = evt.value();
                    search_term.set(new_value.clone());
                    show_dropdown.set(!new_value.is_empty());
                    if new_value.is_empty() {
                        highlighted_index.set(None);
                    }
                },
                onfocusin: move |_| {
                    if !search_term.read().is_empty() && !filtered_items.read().is_empty() {
                        show_dropdown.set(true);
                    }
                },
                onblur: move |_| {
                    let mut show_dropdown_signal = show_dropdown;
                    spawn(async move {
                        gloo_timers::future::TimeoutFuture::new(150).await;
                        show_dropdown_signal.set(false);
                    });
                },
                onkeydown: handle_keyboard
            }

            if *show_dropdown.read() && !filtered_items.read().is_empty() {
                div {
                    class: "absolute z-10 w-full mt-2 panel border-2 shadow-lg max-h-60 overflow-y-auto",
                    ul { class: "py-2",
                        for (idx, item) in filtered_items.read().iter().enumerate() {
                            {
                                let item_clone = item.clone();
                                let is_highlighted = *highlighted_index.read() == Some(idx);
                                let item_id = format!("search-item-{idx}");
                                let highlight_class = if is_highlighted {
                                    "panel-elevated"
                                } else {
                                    "hover:panel-elevated transition-all duration-100"
                                };

                                rsx! {
                                    li {
                                        id: "{item_id}",
                                        key: "{(props.get_key)(item)}",
                                        class: "cursor-pointer mx-2 mb-1 rounded-md {highlight_class}",
                                        style: "outline: none !important; box-shadow: none !important; transition: background-color 0.05s ease !important;",
                                        tabindex: "-1",
                                        onmousedown: move |_| handle_select(item_clone.clone()),
                                        onmouseenter: move |_| highlighted_index.set(Some(idx)),
                                        {(props.render_item)(item)}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
