//! Fixed-format sample application messages for object-controller examples.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationMessageError {
    BufferTooSmall,
    InvalidLength,
    UnknownMessageType,
    UnknownAspect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalAspect {
    Red,
    Green,
    GreenRequested,
    Unknown,
}

impl SignalAspect {
    fn code(self) -> u8 {
        match self {
            Self::Red => 0,
            Self::Green => 1,
            Self::GreenRequested => 2,
            Self::Unknown => 255,
        }
    }

    fn from_code(code: u8) -> Result<Self, ApplicationMessageError> {
        match code {
            0 => Ok(Self::Red),
            1 => Ok(Self::Green),
            2 => Ok(Self::GreenRequested),
            255 => Ok(Self::Unknown),
            _ => Err(ApplicationMessageError::UnknownAspect),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationMessage {
    SignalStatus {
        signal_id: u16,
        aspect: SignalAspect,
    },
    MovementAuthority {
        signal_id: u16,
        allow_green: bool,
        reason_code: u8,
    },
    Ping {
        counter: u32,
    },
    Pong {
        counter: u32,
    },
}

impl ApplicationMessage {
    pub const MAX_ENCODED_LEN: usize = 5;

    pub fn encode(&self, output: &mut [u8]) -> Result<usize, ApplicationMessageError> {
        match *self {
            Self::SignalStatus { signal_id, aspect } => {
                let buffer = output
                    .get_mut(..4)
                    .ok_or(ApplicationMessageError::BufferTooSmall)?;
                buffer[0] = 1;
                buffer[1..3].copy_from_slice(&signal_id.to_le_bytes());
                buffer[3] = aspect.code();
                Ok(4)
            }
            Self::MovementAuthority {
                signal_id,
                allow_green,
                reason_code,
            } => {
                let buffer = output
                    .get_mut(..5)
                    .ok_or(ApplicationMessageError::BufferTooSmall)?;
                buffer[0] = 2;
                buffer[1..3].copy_from_slice(&signal_id.to_le_bytes());
                buffer[3] = u8::from(allow_green);
                buffer[4] = reason_code;
                Ok(5)
            }
            Self::Ping { counter } => {
                let buffer = output
                    .get_mut(..5)
                    .ok_or(ApplicationMessageError::BufferTooSmall)?;
                buffer[0] = 3;
                buffer[1..5].copy_from_slice(&counter.to_le_bytes());
                Ok(5)
            }
            Self::Pong { counter } => {
                let buffer = output
                    .get_mut(..5)
                    .ok_or(ApplicationMessageError::BufferTooSmall)?;
                buffer[0] = 4;
                buffer[1..5].copy_from_slice(&counter.to_le_bytes());
                Ok(5)
            }
        }
    }

    pub fn decode(input: &[u8]) -> Result<Self, ApplicationMessageError> {
        let message_type = *input
            .first()
            .ok_or(ApplicationMessageError::InvalidLength)?;
        match message_type {
            1 => {
                if input.len() != 4 {
                    return Err(ApplicationMessageError::InvalidLength);
                }
                Ok(Self::SignalStatus {
                    signal_id: u16::from_le_bytes([input[1], input[2]]),
                    aspect: SignalAspect::from_code(input[3])?,
                })
            }
            2 => {
                if input.len() != 5 {
                    return Err(ApplicationMessageError::InvalidLength);
                }
                Ok(Self::MovementAuthority {
                    signal_id: u16::from_le_bytes([input[1], input[2]]),
                    allow_green: input[3] != 0,
                    reason_code: input[4],
                })
            }
            3 => {
                if input.len() != 5 {
                    return Err(ApplicationMessageError::InvalidLength);
                }
                Ok(Self::Ping {
                    counter: u32::from_le_bytes([input[1], input[2], input[3], input[4]]),
                })
            }
            4 => {
                if input.len() != 5 {
                    return Err(ApplicationMessageError::InvalidLength);
                }
                Ok(Self::Pong {
                    counter: u32::from_le_bytes([input[1], input[2], input[3], input[4]]),
                })
            }
            _ => Err(ApplicationMessageError::UnknownMessageType),
        }
    }
}

pub fn movement_authority_for_signal(signal_id: u16, aspect: SignalAspect) -> ApplicationMessage {
    match aspect {
        SignalAspect::GreenRequested | SignalAspect::Green => {
            ApplicationMessage::MovementAuthority {
                signal_id,
                allow_green: true,
                reason_code: 0,
            }
        }
        SignalAspect::Red | SignalAspect::Unknown => ApplicationMessage::MovementAuthority {
            signal_id,
            allow_green: false,
            reason_code: 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplicationMessage, ApplicationMessageError, SignalAspect, movement_authority_for_signal,
    };

    fn round_trip(message: ApplicationMessage) {
        let mut buffer = [0u8; ApplicationMessage::MAX_ENCODED_LEN];
        let len = message.encode(&mut buffer).unwrap();
        assert_eq!(ApplicationMessage::decode(&buffer[..len]), Ok(message));
    }

    #[test]
    fn signal_status_encodes_and_decodes() {
        round_trip(ApplicationMessage::SignalStatus {
            signal_id: 1,
            aspect: SignalAspect::Red,
        });
        round_trip(ApplicationMessage::SignalStatus {
            signal_id: 2,
            aspect: SignalAspect::GreenRequested,
        });
    }

    #[test]
    fn movement_authority_encodes_and_decodes() {
        round_trip(ApplicationMessage::MovementAuthority {
            signal_id: 1,
            allow_green: true,
            reason_code: 0,
        });
    }

    #[test]
    fn ping_and_pong_encode_and_decode() {
        round_trip(ApplicationMessage::Ping { counter: 7 });
        round_trip(ApplicationMessage::Pong { counter: 7 });
    }

    #[test]
    fn malformed_application_messages_are_rejected() {
        assert_eq!(
            ApplicationMessage::decode(&[]),
            Err(ApplicationMessageError::InvalidLength)
        );
        assert_eq!(
            ApplicationMessage::decode(&[99]),
            Err(ApplicationMessageError::UnknownMessageType)
        );
        assert_eq!(
            ApplicationMessage::decode(&[1, 0, 0, 3]),
            Err(ApplicationMessageError::UnknownAspect)
        );
        assert_eq!(
            ApplicationMessage::decode(&[3, 1]),
            Err(ApplicationMessageError::InvalidLength)
        );
    }

    #[test]
    fn interlocking_logic_maps_signal_aspects_to_authority() {
        assert_eq!(
            movement_authority_for_signal(1, SignalAspect::Red),
            ApplicationMessage::MovementAuthority {
                signal_id: 1,
                allow_green: false,
                reason_code: 1,
            }
        );
        assert_eq!(
            movement_authority_for_signal(1, SignalAspect::GreenRequested),
            ApplicationMessage::MovementAuthority {
                signal_id: 1,
                allow_green: true,
                reason_code: 0,
            }
        );
    }
}
