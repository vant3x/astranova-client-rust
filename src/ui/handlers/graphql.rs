use crate::ui::app::{AstraNovaApp, Message};
use crate::ui::views::graphql_view;
use iced::Task;

pub fn handle_message(app: &mut AstraNovaApp, msg: graphql_view::Message) -> Task<Message> {
    match msg {
        graphql_view::Message::SendRequest => {
            let mut temp_view = app.graphql_view.clone();
            if let Some(env) = &app.active_environment {
                temp_view.apply_environment(env);
            }

            match temp_view.build_request() {
                Ok(_graphql_request) => {
                    let http_request = temp_view.build_http_request();
                    app.graphql_view.update(graphql_view::Message::SetLoading);

                    let http_client = if http_request.config.proxy_url.is_some()
                        || !http_request.config.verify_ssl
                    {
                        match crate::http_client::client::build_client(&http_request.config) {
                            Ok(c) => c,
                            Err(e) => {
                                log::error!("Failed to build custom client: {}", e);
                                app.http_client.clone()
                            }
                        }
                    } else {
                        app.http_client.clone()
                    };

                    Task::perform(
                        async move {
                            let response = crate::http_client::client::send_request(
                                &http_client,
                                http_request,
                            )
                            .await;

                            match response {
                                Ok(http_response) => {
                                    let graphql_response: crate::protocols::graphql::GraphQLResponse =
                                        serde_json::from_str(&http_response.body)
                                            .unwrap_or_else(|_| crate::protocols::graphql::GraphQLResponse {
                                                data: None,
                                                errors: vec![crate::protocols::graphql::GraphQLError {
                                                    message: format!(
                                                        "Failed to parse GraphQL response: {}",
                                                        &http_response.body[..http_response.body.len().min(200)]
                                                    ),
                                                    locations: vec![],
                                                    path: vec![],
                                                    extensions: None,
                                                }],
                                            });

                                    Ok((
                                        graphql_response,
                                        http_response.status,
                                        http_response.headers,
                                        http_response.duration,
                                        http_response.size,
                                    ))
                                }
                                Err(e) => Err(e),
                            }
                        },
                        move |result| Message::GraphQLMsg(graphql_view::Message::ResponseReceived(
                            result,
                        )),
                    )
                }
                Err(e) => {
                    app.graphql_view
                        .update(graphql_view::Message::ResponseReceived(Err(e)));
                    Task::none()
                }
            }
        }
        other => {
            app.graphql_view.update(other);
            Task::none()
        }
    }
}
