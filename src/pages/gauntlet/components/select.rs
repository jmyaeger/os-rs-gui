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
            // dioxus-primitives now takes the controlled value as a signal, and
            // the placeholder moved onto SelectValue.
            value: Some(value.into()),
            on_value_change: move |new_val: Option<String>| {
                value.set(new_val.clone());
                if let (Some(cb), Some(val)) = (&on_change, new_val) {
                    cb.call(val);
                }
            },

            // Wrapper prevents blur from firing before click toggle
            div { onmousedown: |e: MouseEvent| e.prevent_default(),
                SelectTrigger { class: "w-full input-field flex justify-between items-center gap-2 text-left",
                    SelectValue { placeholder: "{placeholder}" }
                    svg {
                        class: "w-3 h-3 ml-1 shrink-0 text-foreground",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        polyline { points: "6 9 12 15 18 9" }
                    }
                }
            }

            SelectList { class: "picker-menu absolute w-full mt-1 max-h-48 overflow-auto",
                for (idx , option) in options.iter().enumerate() {
                    SelectOption::<String> {
                        index: idx,
                        value: option.clone(),
                        class: "picker-option py-1.5 px-2 text-sm",
                        "{option}"
                    }
                }
            }
        }
    }
}
