use std::borrow::Borrow;
use std::rc::Rc;
use std::time::Duration;

use futures::{Future};
use futures::future::{result};
use tokio_core::reactor::{Handle, Timeout};

use telegram_bot_raw::{Request, ResponseType};

use connector::{Connector, default_connector};
use errors::Error;
use future::{TelegramFuture, NewTelegramFuture};
use stream::{NewUpdatesStream, UpdatesStream};
use std::cell::RefCell;

use futures::Stream;
use telegram_bot_raw::Update;
use std::collections::{HashSet, HashMap};
use telegram_bot_raw::{ChatId};
use telegram_bot_raw::UpdateKind::*;


/// Main type for sending requests to the Telegram bot API.
#[derive(Clone)]
pub struct Api {
    inner: Rc<ApiInner>,
}

struct ApiInner {
    handle: Handle,
    snd: Vec<(String, Box<Connector>)>,
    sndhp: Vec<(String, Box<Connector>)>,
    rcv: Vec<(String, Box<Connector>)>,
    rr: RefCell<usize>,
    rrhp: RefCell<usize>,
    rooms: RefCell<HashMap<usize, HashSet<ChatId>>>
}

#[derive(Debug)]
pub enum ConnectorConfig {
    Default,
    Specified(Box<Connector>)
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        ConnectorConfig::Default
    }
}

impl ConnectorConfig {
    pub fn new(connector: Box<Connector>) -> Self {
        ConnectorConfig::Specified(connector)
    }

    pub fn take(self, handle: &Handle) -> Result<Box<Connector>, Error> {
        match self {
            ConnectorConfig::Default => default_connector(&handle),
            ConnectorConfig::Specified(connector) => Ok(connector)
        }
    }
}

/// Configuration for an `Api`.
#[derive(Debug)]
pub struct Config {
    token: String,
    connector: ConnectorConfig,
}

pub enum SendType {
    LOWPRIO,
    HIGHPRIO,
    DETERMINISTIC,
}

impl Config {
    /// Set connector type for an `Api`.
    pub fn connector(self, connector: Box<Connector>) -> Config {
        Config {
            token: self.token,
            connector: ConnectorConfig::new(connector),
        }
    }

    /// Create new `Api` instance.
    pub fn build<H: Borrow<Handle>>(self, _handle: H) -> Result<Api, Error> {
        unimplemented!();
    }
}

impl Api {
    /// Start construction of the `Api` instance.
    ///
    /// # Examples
    ///
    /// Using default connector.
    ///
    /// ```rust
    /// # extern crate telegram_bot;
    /// # extern crate tokio_core;
    /// use telegram_bot::Api;
    /// use tokio_core::reactor::Core;
    ///
    /// # fn main() {
    /// let core = Core::new().unwrap();
    /// # let telegram_token = "token";
    /// let api = Api::configure(telegram_token).build(core.handle()).unwrap();
    /// # }
    /// ```
    ///
    /// Using custom connector.
    ///
    ///
    /// ```rust
    /// # extern crate telegram_bot;
    /// # extern crate tokio_core;
    /// # #[cfg(feature = "hyper_connector")]
    /// # fn main() {
    /// use telegram_bot::Api;
    /// use telegram_bot::connector::hyper;
    /// use tokio_core::reactor::Core;
    ///
    /// let core = Core::new().unwrap();
    /// # let telegram_token = "token";
    /// let api = Api::configure(telegram_token)
    ///     .connector(hyper::default_connector(&core.handle()).unwrap())
    ///     .build(core.handle()).unwrap();
    /// # }
    ///
    /// # #[cfg(not(feature = "hyper_connector"))]
    /// # fn main() {}
    /// ```
    pub fn configure<T: AsRef<str>>(_token: T) -> Config {
        unimplemented!();
    }

    pub fn build_multi<T: AsRef<str>, H: Borrow<Handle>>(tokens: &[T], handle: H) -> Result<Api, Error> {
        let handle = handle.borrow().clone();
        let mut v1 : Vec<(String, Box<Connector>)> = Vec::new();
        let mut v2 : Vec<(String, Box<Connector>)> = Vec::new();
        let mut v3 : Vec<(String,Box<Connector>)> = Vec::new();

        for t in tokens.iter() {
            let connector : ConnectorConfig = Default::default();
            v1.push((t.as_ref().into(), connector.take(&handle)?));
            let connector : ConnectorConfig = Default::default();
            v2.push((t.as_ref().into(), connector.take(&handle)?));
            let connector : ConnectorConfig = Default::default();
            v3.push((t.as_ref().into(),connector.take(&handle)?));
        }

        Ok(Api {
            inner: Rc::new(ApiInner {
                snd : v1,
                sndhp: v3,
                rcv: v2,
                handle: handle,
                rr: RefCell::new(0),
                rrhp: RefCell::new(0),
                rooms: RefCell::new(HashMap::new())
            }),
        })
    }

    /// Create a stream which produces updates from the Telegram server.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate futures;
    /// # extern crate telegram_bot;
    /// # extern crate tokio_core;
    /// # use telegram_bot::Api;
    /// # use tokio_core::reactor::Core;
    /// # fn main() {
    /// # let core = Core::new().unwrap();
    /// # let api: Api = Api::configure("token").build(core.handle()).unwrap();
    /// use futures::Stream;
    ///
    /// let future = api.stream().for_each(|update| {
    ///     println!("{:?}", update);
    ///     Ok(())
    /// });
    /// # }
    /// ```
    pub fn stream(&self) -> Box<Stream<Item=Update,Error=Error>> {
        let mut s: Box<Stream<Item=(usize, Update), Error=Error>> = Box::new(UpdatesStream::new(self.clone(), self.inner.handle.clone(), 0));
        for i in 1..self.inner.rcv.len() {
            let si = UpdatesStream::new(self.clone(), self.inner.handle.clone(), i);
            s = Box::new(s.select(si));
        }
    
        let mut hs = [HashSet::new(), HashSet::new()];
        let mut p = 0;

        let api = self.inner.clone();
        Box::new(s.filter(move |&(idx, ref update)| {
            let chatmessageid = match update.kind {
                Message(ref m) => {
                    (m.chat.id(), m.id.to_string())
                },
                EditedMessage(ref m) => {
                    (m.chat.id(), m.id.to_string())
                },
                ChannelPost(ref m) => {
                    (ChatId::from(m.chat.id), m.id.to_string())
                },
                EditedChannelPost(ref m) => {
                    (ChatId::from(m.chat.id), m.id.to_string())
                },
                CallbackQuery(ref data) => {
                    (ChatId::from(match data.chat_instance.parse() {
                        Ok(number) => number,
                        Err(_) => 0
                    }), format!("{:?}", data.id))
                },
                Unknown(ref m) => {
                    (ChatId::from(0), m.update_id.to_string())
                }
            };

            api.rooms.borrow_mut().entry(idx).or_insert_with(|| HashSet::new()).insert(chatmessageid.0);

            let dup = hs[0].contains(&chatmessageid) || hs[1].contains(&chatmessageid);

            hs[p].insert(chatmessageid);

            if hs[p].len() > 10000 {
                p = 1 - p;
                hs[p].clear();
            }

            !dup
        }).map(|(_, update)| update))
    }

    pub fn rcv<Req: Request>(&self, request: Req, idx:usize)
        -> TelegramFuture<<Req::Response as ResponseType>::Type> {
        let request = request.serialize()
            .map_err(From::from);

        let request = result(request);

        let api = self.clone();
        let response = request.and_then(move |request| {
            let ref token = api.inner.rcv[idx].0;
            api.inner.rcv[idx].1.request(token, request)
        });

        let future = response.and_then(move |response| {
            let result = Req::Response::deserialize(response.clone()).map_err(From::from);

            match result {
                Ok(_) => {},
                Err(_) => {
                    use std::str;

                    if let Some(ref body) = response.body {
                        println!("Error! Response body: {:?}", str::from_utf8(body));
                    }
                }
            }

            result
        });

        TelegramFuture::new(Box::new(future))
    }

    pub fn rcv_timeout<Req: Request>(&self, request: Req, duration: Duration, idx: usize)
        -> TelegramFuture<Option<<Req::Response as ResponseType>::Type>> {
        let timeout_future = result(Timeout::new(duration, &self.inner.handle))
            .flatten().map_err(From::from).map(|()| None);
        let send_future = self.rcv(request,idx).map(|resp| Some(resp));

        let future = timeout_future.select(send_future)
            .map(|(item, _next)| item)
            .map_err(|(item, _next)| item);

        TelegramFuture::new(Box::new(future))
    }

    /// Send a request to the Telegram server and do not wait for a response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate futures;
    /// # extern crate telegram_bot;
    /// # extern crate tokio_core;
    /// # use futures::Future;
    /// # use telegram_bot::{Api, GetMe, ChatId};
    /// # use telegram_bot::prelude::*;
    /// # use tokio_core::reactor::Core;
    /// #
    /// # fn main() {
    /// # let core = Core::new().unwrap();
    /// # let telegram_token = "token";
    /// # let api = Api::configure(telegram_token).build(core.handle()).unwrap();
    /// # if false {
    /// let chat = ChatId::new(61031);
    /// api.spawn(chat.text("Message"))
    /// # }
    /// # }
    pub fn spawn<Req: Request>(&self, request: Req, chatid: ChatId, send_type: SendType) {
        self.inner.handle.spawn(self.send(request, chatid, send_type).then(|_| Ok(())))
    }

    /// Send a request to the Telegram server and wait for a response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate futures;
    /// # extern crate telegram_bot;
    /// # extern crate tokio_core;
    /// # use futures::Future;
    /// # use telegram_bot::{Api, GetMe};
    /// # use tokio_core::reactor::Core;
    /// #
    /// # fn main() {
    /// # let core = Core::new().unwrap();
    /// # let telegram_token = "token";
    /// # let api = Api::configure(telegram_token).build(core.handle()).unwrap();
    /// # if false {
    /// let future = api.send(GetMe);
    /// future.and_then(|me| Ok(println!("{:?}", me)));
    /// # }
    /// # }
    /// ```
    pub fn send<Req: Request>(&self, request: Req, chatid: ChatId, send_type: SendType)
        -> TelegramFuture<<Req::Response as ResponseType>::Type> {
        let request = request.serialize()
            .map_err(From::from);

        let request = result(request);

        let api = self.clone();

        let idxs: Vec<usize> = self.inner.rooms.borrow().iter().filter(|&(_, hs)| {
            hs.contains(&chatid)
        }).map(|(idx, _)| idx).cloned().collect();

        let srr = match send_type {
            SendType::HIGHPRIO => {
                &self.inner.rrhp
            },
            SendType::DETERMINISTIC => {
                &self.inner.rr
            },
            SendType::LOWPRIO => {
                &self.inner.rr
            },
        };

        let rx = match send_type {
            SendType::DETERMINISTIC => {
                if idxs.len() > 0 {
                    idxs[0]
                } else {
                    0
                }
            },
            _ => {
                if idxs.len() > 0 {
                    *srr.borrow_mut() += 1;

                    idxs[*srr.borrow() % idxs.len()]
                } else {
                    0
                }
            },
        };

        let response = request.and_then(move |request| {
            let pair = match send_type {
                SendType::HIGHPRIO => {
                    &api.inner.sndhp[rx]
                },
                _ => {
                    &api.inner.snd[rx]
                },
            };

            let ref token = pair.0;
            pair.1.request(token, request)
        });

        let future = response.and_then(move |response| {
            Req::Response::deserialize(response).map_err(From::from)
        });

        TelegramFuture::new(Box::new(future))
    }

    /// Send a request to the Telegram server and wait for a response, timing out after `duration`.
    /// Future will resolve to `None` if timeout fired.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate futures;
    /// # extern crate telegram_bot;
    /// # extern crate tokio_core;
    /// # use futures::Future;
    /// # use telegram_bot::{Api, GetMe};
    /// # use tokio_core::reactor::Core;
    /// #
    /// # fn main() {
    /// # let core = Core::new().unwrap();
    /// # let telegram_token = "token";
    /// # let api = Api::configure(telegram_token).build(core.handle()).unwrap();
    /// # if false {
    /// use std::time::Duration;
    ///
    /// let future = api.send_timeout(GetMe, Duration::from_secs(5));
    /// future.and_then(|me| Ok(assert!(me.is_some())));
    /// # }
    /// # }
    /// ```
    pub fn send_timeout<Req: Request>(&self, request: Req, duration: Duration, chatid: ChatId, send_type: SendType)
        -> TelegramFuture<Option<<Req::Response as ResponseType>::Type>> {
        let timeout_future = result(Timeout::new(duration, &self.inner.handle))
            .flatten().map_err(From::from).map(|()| None);
        let send_future = self.send(request, chatid, send_type).map(|resp| Some(resp));

        let future = timeout_future.select(send_future)
            .map(|(item, _next)| item)
            .map_err(|(item, _next)| item);

        TelegramFuture::new(Box::new(future))
    }
}
