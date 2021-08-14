use futures_util::stream::StreamExt;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions,
    QueueDeclareOptions,
};
use lapin::{
    publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties,
};

use crate::config;
use crate::types;

#[derive(Debug, Clone)]
struct RpcClient {
    channel: lapin::Channel,
    callback_queue: lapin::Queue,
}

impl RpcClient {
    pub fn try_new() -> Result<RpcClient, Box<dyn std::error::Error>> {
        async_global_executor::block_on(async {
            let conn = Connection::connect(
                &config::AMQP_URL,
                ConnectionProperties::default().with_default_executor(8),
            )
            .await?;

            let channel = conn.create_channel().await?;
            let _queue = channel
                .queue_declare(
                    &config::AMQP_EXCHANGE,
                    QueueDeclareOptions::default(),
                    FieldTable::default(),
                )
                .await?;

            let q_options = QueueDeclareOptions {
                exclusive: true,
                ..QueueDeclareOptions::default()
            };
            let callback_queue = channel
                .queue_declare("", q_options, FieldTable::default())
                .await?;

            Ok(RpcClient {
                channel,
                callback_queue,
            })
        })
    }

    pub fn call(&self, args: types::Args) -> Result<types::Result, Box<dyn std::error::Error>> {
        async_global_executor::block_on(async {
            let payload = serde_json::to_vec(&args)?;

            let correlation_id = format!("{}", uuid::Uuid::new_v4().to_hyphenated());
            let properties = BasicProperties::default()
                .with_correlation_id(correlation_id.clone().into())
                .with_reply_to(self.callback_queue.name().clone());

            let confirm = self
                .channel
                .basic_publish(
                    "",
                    &config::AMQP_EXCHANGE,
                    BasicPublishOptions::default(),
                    payload.to_vec(),
                    properties,
                )
                .await?
                .await?;
            assert_eq!(confirm, Confirmation::NotRequested);

            let consumer_tag = format!("ctag-{}", uuid::Uuid::new_v4());
            let mut callback_consumer = self
                .channel
                .basic_consume(
                    self.callback_queue.name().clone().as_str(),
                    &consumer_tag,
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await?;

            loop {
                let delivery = callback_consumer.next().await.unwrap();
                let (_, delivery) = delivery.expect("error in consumer");
                let corr_id_option = delivery.properties.correlation_id().clone();
                let corr_id;
                match corr_id_option {
                    Some(c) => corr_id = c.clone(),
                    None => {
                        continue;
                    }
                }
                if corr_id.as_str() != correlation_id {
                    delivery
                        .nack(BasicNackOptions::default())
                        .await
                        .expect("nack");
                    continue;
                }

                let res_result = serde_json::from_slice::<types::Result>(&delivery.data);
                let res: types::Result;

                match res_result {
                    Ok(r) => {
                        res = r;
                    }
                    Err(_) => {
                        println!("Bad response format.");
                        delivery.ack(BasicAckOptions::default()).await.expect("ack");
                        continue;
                    }
                }

                delivery.ack(BasicAckOptions::default()).await.expect("ack");
                break Ok(res);
            }
        })
    }
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RpcClient::try_new()?;
    let args = types::Args {
        string: "hello world".into(),
    };
    let res = client.call(args)?;
    println!("{:?}", res.length);
    Ok(())
}
