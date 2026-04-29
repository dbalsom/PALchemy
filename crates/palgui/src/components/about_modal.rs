use crate::ipc::get_build_info;
use leptos::prelude::*;
use palcore::{AppError, BuildInfo};
use stylance::import_style;

import_style!(style, "about_modal.module.scss");

#[component]
pub fn AboutModal(show_about_modal: RwSignal<bool>) -> impl IntoView {
    let build_info: LocalResource<Result<BuildInfo, AppError>> =
        LocalResource::new(|| async move { get_build_info().await });

    view! {
        <Show when=move || show_about_modal.get()>
            <div
                class=style::modal_overlay
                on:click=move |_| show_about_modal.set(false)
                on:keydown=move |event| {
                    if event.key() == "Escape" {
                        show_about_modal.set(false);
                    }
                }
            >
                <div
                    class=style::modal_content
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="about-title"
                    aria-describedby="about-summary"
                    tabindex="-1"
                    on:click=move |event| event.stop_propagation()
                >
                    <div class=style::modal_header>
                        <div class=style::logo_section>
                            <img src="/assets/img/palchemy_logo.png" alt="PALchemy Logo" class=style::logo_icon/>
                            <h1 id="about-title">
                                <span class=style::pal_text>"PAL"</span>
                                <span class=style::chemy_text>"chemy"</span>
                            </h1>
                        </div>
                        <button
                            class=style::close_icon
                            aria-label="Close about dialog"
                            autofocus
                            on:click=move |_| show_about_modal.set(false)
                        >
                            "x"
                        </button>
                    </div>
                    <div class=style::modal_body>
                        <div class=style::copyright_section>
                            <p id="about-summary" class="sr_only">
                                "About PALchemy, including project information and build details."
                            </p>
                            <p>"Copyright 2026 Daniel Balsom"</p>
                            <p>"This is free software, licensed under the terms of the MIT License."</p>
                            <p>
                                <a href="https://github.com/dbalsom/palchemy" target="_blank" class=style::github_link>
                                    "View on GitHub"
                                </a>
                            </p>
                        </div>

                        <div class=style::build_info_container>
                            <h3>"Build Information"</h3>
                            <div class=style::build_info_scroll>
                                <Suspense fallback=move || view! { <p>"Loading build info..."</p> }>
                                    {move || {
                                        build_info.get().map(|result| {
                                            match result {
                                                Ok(info) => {
                                                    view! {
                                                        <pre class=style::build_text>
                                                            {format!("Package:    {}\n", info.pkg_name)}
                                                            {format!("Version:    {}\n", info.pkg_version)}
                                                            {format!("Authors:    {}\n", info.pkg_authors)}
                                                            {format!("License:    {}\n", info.pkg_license)}
                                                            {format!("Repository: {}\n", info.pkg_repository)}
                                                            {format!("Target:     {}\n", info.target)}
                                                            {format!("Host:       {}\n", info.host)}
                                                            {format!("Profile:    {}\n", info.profile)}
                                                            {format!("Rustc:      {}\n", info.rustc_version)}
                                                            {format!("Features:   {:?}\n", info.features)}
                                                            {format!("Git Ver:    {:?}\n", info.git_version)}
                                                            {format!("Git Dirty:  {:?}\n", info.git_dirty)}
                                                            {format!("Git Commit: {:?}\n", info.git_commit_hash)}
                                                            {format!("CI Plat:    {:?}\n", info.ci_platform)}
                                                            {format!("Drivers:    {}\n", info.available_drivers.join(", "))}
                                                            {format!("Chip Defs:  {}\n", info.loaded_chip_definitions)}
                                                        </pre>
                                                    }
                                                        .into_any()
                                                }
                                                Err(error) => {
                                                    view! {
                                                        <p class="err-text">{format!("Error loading build info: {error}")}</p>
                                                    }
                                                        .into_any()
                                                }
                                            }
                                        })
                                    }}
                                </Suspense>
                            </div>
                        </div>
                    </div>
                    <div class=style::modal_footer>
                        <button class="btn secondary" on:click=move |_| show_about_modal.set(false)>
                            "Close"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
