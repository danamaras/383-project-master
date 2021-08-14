use futures_util::stream::StreamExt;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions,
};
use lapin::{types::FieldTable, BasicProperties, Connection, ConnectionProperties};

use crate::config;
use crate::types;

fn get_length(a: types::Args) -> types::Result {
    types::Result {
        length: a.string.len(),
    }
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    async_global_executor::block_on(async {
        let conn = Connection::connect(
            &config::AMQP_URL,
            ConnectionProperties::default().with_default_executor(8),
        )
        .await?;

        let channel = conn.create_channel().await?;
        let consumer_tag = format!("ctag-{}", uuid::Uuid::new_v4());

        let _queue = channel
            .queue_declare(
                &config::AMQP_EXCHANGE,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let mut consumer = channel
            .basic_consume(
                config::AMQP_EXCHANGE,
                &consumer_tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        println!("Listening");

        while let Some(delivery) = consumer.next().await {
            let (_, delivery) = delivery.expect("error in consumer");

            println!("received {} byte request", delivery.data.len());

            let reply_to_option = delivery.properties.reply_to().clone();
            let reply_to;
            match reply_to_option {
                Some(r) => {
                    reply_to = r.clone();
                }
                None => {
                    println!("No reply_to.");
                    delivery.ack(BasicAckOptions::default()).await.expect("ack");
                    continue;
                }
            }

            let corr_id_option = delivery.properties.correlation_id().clone();
            let correlation_id;
            match corr_id_option {
                Some(c) => {
                    correlation_id = c.clone();
                }
                None => {
                    println!("No correlation_id.");
                    delivery.ack(BasicAckOptions::default()).await.expect("ack");
                    continue;
                }
            }

            let arg_result = serde_json::from_slice::<types::Args>(&delivery.data);
            let args;
            match arg_result {
                Ok(a) => {
                    args = a;
                }
                Err(_) => {
                    println!("Bad request.");
                    delivery.ack(BasicAckOptions::default()).await.expect("ack");
                    continue;
                }
            }

            let result = get_length(args);
            let response = serde_json::to_vec(&result)?;
            channel
                .basic_publish(
                    "",
                    reply_to.as_str(),
                    BasicPublishOptions::default(),
                    response,
                    BasicProperties::default().with_correlation_id(correlation_id),
                )
                .await
                .expect("basic_publish");
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
        }
        Ok(())
    })
}
