use serde_json::{Value, json};

use super::*;
use crate::remnawave::test_support::*;

#[tokio::test]
async fn lists_owned_devices_and_preserves_nullable_metadata() {
    let mut item = device_json(7, "phone");
    item["deviceModel"] = Value::Null;
    item["osVersion"] = Value::Null;
    item["platform"] = Value::Null;
    let (client, server) = mock_api(vec![
        users_response(vec![user(7, 42)]),
        devices_response(vec![item]),
    ]);
    let list = DeviceService::new(client).list(42).await.unwrap();
    assert_eq!(list.limit, Some(5));
    assert_eq!(list.devices.len(), 1);
    assert!(list.devices[0].device_model.is_none());
    let requests = server.join().unwrap();
    assert!(
        requests[0]
            .headers
            .starts_with("GET /api/users/stream?telegramId=42&size=10 ")
    );
    assert!(requests[1].headers.starts_with("GET /api/hwid/devices/7 "));
    assert!(requests.iter().all(|request| {
        request
            .headers
            .to_lowercase()
            .contains("authorization: bearer test-token")
    }));
}

#[tokio::test]
async fn rejects_missing_ambiguous_and_foreign_subscriptions() {
    for (users, expected) in [
        (vec![], "missing"),
        (vec![user(7, 42), user(8, 42)], "multiple"),
        (vec![user(7, 99)], "foreign"),
    ] {
        let (client, server) = mock_api(vec![users_response(users)]);
        let result = DeviceService::new(client).list(42).await;
        match expected {
            "missing" => assert!(matches!(result, Err(DeviceError::SubscriptionNotFound))),
            "multiple" => assert!(matches!(result, Err(DeviceError::MultipleSubscriptions))),
            _ => assert!(matches!(result, Err(DeviceError::OwnershipMismatch))),
        }
        assert_eq!(server.join().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn refuses_foreign_device_even_with_a_valid_token() {
    let (client, server) = mock_api(vec![
        users_response(vec![user(7, 42)]),
        devices_response(vec![device_json(8, "foreign")]),
    ]);
    let result = DeviceService::new(client)
        .delete(42, &device_token(&device(8, "foreign")))
        .await;
    assert!(matches!(result, Err(DeviceError::OwnershipMismatch)));
    assert_eq!(server.join().unwrap().len(), 2);
}

#[tokio::test]
async fn deletion_uses_stable_identity_after_list_reordering_and_rejects_replay() {
    let token = device_token(&device(7, "phone"));
    let (client, server) = mock_api(vec![
        users_response(vec![user(7, 42)]),
        devices_response(vec![device_json(7, "phone"), device_json(7, "laptop")]),
        users_response(vec![user(7, 42)]),
        devices_response(vec![device_json(7, "laptop"), device_json(7, "phone")]),
        devices_response(vec![device_json(7, "laptop")]),
        users_response(vec![user(7, 42)]),
        devices_response(vec![device_json(7, "laptop")]),
    ]);
    let service = DeviceService::new(client);
    assert_eq!(service.get(42, &token).await.unwrap().hwid, "phone");
    let remaining = service.delete(42, &token).await.unwrap();
    assert_eq!(remaining.devices.len(), 1);
    assert_eq!(remaining.devices[0].hwid, "laptop");
    assert!(matches!(
        service.delete(42, &token).await,
        Err(DeviceError::DeviceNotFound)
    ));
    let requests = server.join().unwrap();
    let posts: Vec<_> = requests
        .iter()
        .filter(|request| request.headers.starts_with("POST "))
        .collect();
    assert_eq!(posts.len(), 1);
    assert!(
        posts[0]
            .headers
            .starts_with("POST /api/hwid/devices/delete ")
    );
    assert_eq!(
        serde_json::from_str::<Value>(&posts[0].body).unwrap(),
        json!({"userId": 7, "hwid": "phone"})
    );
}

#[tokio::test]
async fn deletion_rechecks_ownership_after_confirmation() {
    let token = device_token(&device(7, "phone"));
    let (client, server) = mock_api(vec![
        users_response(vec![user(7, 42)]),
        devices_response(vec![device_json(7, "phone")]),
        users_response(vec![user(7, 99)]),
    ]);
    let service = DeviceService::new(client);
    service.get(42, &token).await.unwrap();
    assert!(matches!(
        service.delete(42, &token).await,
        Err(DeviceError::OwnershipMismatch)
    ));
    assert!(
        server
            .join()
            .unwrap()
            .iter()
            .all(|request| !request.headers.starts_with("POST "))
    );
}

#[tokio::test]
async fn foreign_token_and_recreated_device_cannot_be_deleted() {
    let mut recreated = device_json(7, "phone");
    recreated["createdAt"] = json!("2026-09-17T00:00:00Z");
    for (token, item) in [
        (device_token(&device(8, "phone")), device_json(7, "phone")),
        (device_token(&device(7, "phone")), recreated),
    ] {
        let (client, server) = mock_api(vec![
            users_response(vec![user(7, 42)]),
            devices_response(vec![item]),
        ]);
        assert!(matches!(
            DeviceService::new(client).delete(42, &token).await,
            Err(DeviceError::DeviceNotFound)
        ));
        assert_eq!(server.join().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn rejects_incomplete_list_and_api_errors_without_reporting_success() {
    let token = device_token(&device(7, "phone"));
    let (client, server) = mock_api(vec![
        users_response(vec![user(7, 42)]),
        (200, json!({"response": {"total": 1, "devices": []}})),
    ]);
    assert!(matches!(
        DeviceService::new(client).delete(42, &token).await,
        Err(DeviceError::IncompleteDeviceList)
    ));
    server.join().unwrap();

    for status in [401, 404, 500] {
        let (client, server) = mock_api(vec![
            users_response(vec![user(7, 42)]),
            devices_response(vec![device_json(7, "phone")]),
            (status, json!({"message": "private response"})),
        ]);
        let result = DeviceService::new(client).delete(42, &token).await;
        assert!(matches!(
            result,
            Err(DeviceError::Remnawave(RemnawaveError::DeviceApi { .. }))
        ));
        server.join().unwrap();
    }
}

#[tokio::test]
async fn refuses_success_response_that_still_contains_the_device() {
    let (client, server) = mock_api(vec![
        users_response(vec![user(7, 42)]),
        devices_response(vec![device_json(7, "phone")]),
        devices_response(vec![device_json(7, "phone")]),
    ]);
    let result = DeviceService::new(client)
        .delete(42, &device_token(&device(7, "phone")))
        .await;
    assert!(matches!(result, Err(DeviceError::DeletionNotApplied)));
    server.join().unwrap();
}

#[tokio::test]
async fn malformed_callback_does_not_contact_api() {
    let (client, server) = mock_api(vec![]);
    let service = DeviceService::new(client);
    for token in ["", "7:phone", "../../", "неверная-кнопка"] {
        assert!(matches!(
            service.delete(42, token).await,
            Err(DeviceError::DeviceNotFound)
        ));
    }
    assert!(server.join().unwrap().is_empty());
}
