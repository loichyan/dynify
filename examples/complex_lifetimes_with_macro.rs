use std::mem::MaybeUninit;

use dynify::Dynify;

#[dynify::dynify]
trait UserCommunication {
    async fn send_sms(&self, phone: &str, code: &str);
    async fn send_email(&self, email: &str, code: &str);
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
