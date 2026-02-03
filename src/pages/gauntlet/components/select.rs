use dioxus::prelude::*;
use dioxus_primitives::select::{
    Select as PrimitiveSelect, SelectList, SelectOption, SelectTrigger, SelectValue,
};

#[component]
pub fn Select(
    options: Vec<String>,
    value: Signal<Option<String>>,
    #[props(default = "Select an option...".to_string())] placeholder: String,
    #[props(default)] on_change: Option<Callback<String>>,
) -> Element {
    rsx! {
        PrimitiveSelect {
            class: "relative w-full",
            value: value(),
            placeholder: "{placeholder}",
            on_value_change: move |new_val: Option<String>| {
                value.set(new_val.clone());
                if let (Some(cb), Some(val)) = (&on_change, new_val) {
                    cb.call(val);
                }
            },

            // Wrapper prevents blur from firing before click toggle
            div {
                onmousedown: |e: MouseEvent| e.prevent_default(),
                SelectTrigger {
                    class: "w-full input-field text-gray-200 py-1.5 px-2 text-sm rounded cursor-pointer flex justify-between items-center transition-all",
                    SelectValue {}
                    svg {
                        class: "w-3 h-3 ml-1 shrink-0 text-gray-300",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        polyline { points: "6 9 12 15 18 9" }
                    }
                }
            }

            SelectList {
                class: "absolute z-10 w-full mt-1 bg-slate-900 border border-slate-700 rounded-lg shadow-lg max-h-48 overflow-auto",
                for (idx, option) in options.iter().enumerate() {
                    SelectOption::<String> {
                        index: idx,
                        value: option.clone(),
                        class: "w-full text-left py-1.5 px-2 text-sm text-gray-300 hover:bg-slate-800 hover:text-white focus:bg-slate-800 focus:text-white focus:outline-none cursor-pointer transition-colors",
                        "{option}"
                    }
                }
            }
        }
    }
}
