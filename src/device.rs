use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::remnawave::{HwidDevice, HwidDevices, RemnawaveClient, RemnawaveError, RemnawaveUser};

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct DeviceService {
    remnawave: RemnawaveClient,
}

pub struct DeviceList {
    pub limit: Option<u32>,
    pub devices: Vec<HwidDevice>,
}

impl DeviceService {
    pub fn new(remnawave: RemnawaveClient) -> Self {
        Self { remnawave }
    }

    async fn owned_user(&self, telegram_id: u64) -> Result<RemnawaveUser, DeviceError> {
        let mut users = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            self.remnawave.find_users_by_telegram_id(telegram_id),
        )
        .await
        .map_err(|_| DeviceError::Timeout)??;

        if users
            .iter()
            .any(|user| user.telegram_id != Some(telegram_id) || user.id <= 0)
        {
            return Err(DeviceError::OwnershipMismatch);
        }

        match users.len() {
            0 => Err(DeviceError::SubscriptionNotFound),
            1 => Ok(users.remove(0)),
            _ => Err(DeviceError::MultipleSubscriptions),
        }
    }

    async fn load(&self, telegram_id: u64) -> Result<(RemnawaveUser, DeviceList), DeviceError> {
        let user = self.owned_user(telegram_id).await?;
        let devices = self.remnawave.get_user_devices(user.id).await?;
        let list = device_list(&user, devices)?;
        Ok((user, list))
    }

    pub async fn list(&self, telegram_id: u64) -> Result<DeviceList, DeviceError> {
        Ok(self.load(telegram_id).await?.1)
    }

    pub async fn get(&self, telegram_id: u64, token: &str) -> Result<HwidDevice, DeviceError> {
        validate_token(token)?;
        let (_, list) = self.load(telegram_id).await?;
        Ok(find_device(&list, token)?.clone())
    }

    pub async fn delete(&self, telegram_id: u64, token: &str) -> Result<DeviceList, DeviceError> {
        validate_token(token)?;
        let (user, list) = self.load(telegram_id).await?;
        let device = find_device(&list, token)?;
        let remaining = self
            .remnawave
            .delete_user_device(user.id, &device.hwid)
            .await?;
        let remaining = device_list(&user, remaining)?;

        if remaining
            .devices
            .iter()
            .any(|item| device_token(item) == token)
        {
            return Err(DeviceError::DeletionNotApplied);
        }

        Ok(remaining)
    }
}

fn device_list(user: &RemnawaveUser, devices: HwidDevices) -> Result<DeviceList, DeviceError> {
    if devices
        .devices
        .iter()
        .any(|device| device.user_id != user.id)
    {
        return Err(DeviceError::OwnershipMismatch);
    }

    if devices.total != devices.devices.len() {
        return Err(DeviceError::IncompleteDeviceList);
    }

    let mut devices = devices.devices;
    devices.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then(left.hwid.cmp(&right.hwid))
    });

    Ok(DeviceList {
        limit: user.hwid_device_limit,
        devices,
    })
}

pub fn device_token(device: &HwidDevice) -> String {
    let mut digest = Sha256::new();
    digest.update(device.user_id.to_be_bytes());
    digest.update((device.hwid.len() as u64).to_be_bytes());
    digest.update(device.hwid.as_bytes());
    digest.update(device.created_at.as_bytes());
    digest.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_token(token: &str) -> Result<(), DeviceError> {
    if token.len() != 32 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DeviceError::DeviceNotFound);
    }
    Ok(())
}

fn find_device<'a>(list: &'a DeviceList, token: &str) -> Result<&'a HwidDevice, DeviceError> {
    let mut matches = list
        .devices
        .iter()
        .filter(|device| device_token(device) == token);
    match (matches.next(), matches.next()) {
        (Some(device), None) => Ok(device),
        _ => Err(DeviceError::DeviceNotFound),
    }
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Remnawave не ответил вовремя")]
    Timeout,
    #[error(transparent)]
    Remnawave(#[from] RemnawaveError),
    #[error("подписка не найдена")]
    SubscriptionNotFound,
    #[error("найдено несколько подписок")]
    MultipleSubscriptions,
    #[error("владелец подписки или устройства не совпадает")]
    OwnershipMismatch,
    #[error("список устройств неполон")]
    IncompleteDeviceList,
    #[error("устройство не найдено или кнопка устарела")]
    DeviceNotFound,
    #[error("Remnawave не подтвердил удаление устройства")]
    DeletionNotApplied,
}
