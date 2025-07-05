use std::future::Future;
use std::mem::MaybeUninit;

use dynify::{from_fn, Dynify, Fn};

trait UserCommunication {
    async fn send_sms(&self, phone: &str, code: &str);
    async fn send_email(&self, email: &str, code: &str);
}

// TODO: implement a proc_macro for such boilerplates?
trait DynUserCommunication {
    fn send_sms<'this, 'phone, 'code, 'ret>(
        &'this self,
        phone: &'phone str,
        code: &'code str,
    ) -> Fn!(&'this Self, &'phone str, &'code str => dyn 'ret + Future<Output = ()>)
    where
        'this: 'ret,
        'phone: 'ret,
        'code: 'ret;

    fn send_email<'this, 'email, 'code, 'ret>(
        &'this self,
        email: &'email str,
        code: &'code str,
    ) -> Fn!(&'this Self, &'email str, &'code str => dyn 'ret + Future<Output = ()>)
    where
        'this: 'ret,
        'email: 'ret,
        'code: 'ret;
}
impl<T: UserCommunication> DynUserCommunication for T {
    fn send_sms<'this, 'phone, 'code, 'ret>(
        &'this self,
        phone: &'phone str,
        code: &'code str,
    ) -> Fn!(&'this Self, &'phone str, &'code str => dyn 'ret + Future<Output = ()>)
    where
        'this: 'ret,
        'phone: 'ret,
        'code: 'ret,
    {
        from_fn!(T::send_sms, self, phone, code)
    }

    fn send_email<'this, 'email, 'code, 'ret>(
        &'this self,
        email: &'email str,
        code: &'code str,
    ) -> Fn!(&'this Self, &'email str, &'code str => dyn 'ret + Future<Output = ()>)
    where
        'this: 'ret,
        'email: 'ret,
        'code: 'ret,
    {
        from_fn!(T::send_email, self, email, code)
    }
}

struct TestUser<'a>(&'a str);
impl UserCommunication for TestUser<'_> {
    async fn send_sms(&self, phone: &str, code: &str) {
        println!(
            "send sms to user({}), phone={}, code={}",
            self.0, phone, code
        );
    }
    async fn send_email(&self, email: &str, code: &str) {
        println!(
            "send email to user({}), email={}, code={}",
            self.0, email, code
        );
    }
}

async fn dynamic_dispatch(conn: &dyn DynUserCommunication) {
    let mut stack = MaybeUninit::<[u8; 16]>::uninit();
    let mut heap = Vec::<MaybeUninit<u8>>::new();
    conn.send_sms("123-456-789", "7519")
        .init2(&mut stack, &mut heap)
        .await;
    conn.send_email("pink@rock.star", "1509")
        .init2(&mut stack, &mut heap)
        .await;
}

#[pollster::main]
async fn main() {
    let user = TestUser("rolling_fancy_2557");
    dynamic_dispatch(&user).await;
}
