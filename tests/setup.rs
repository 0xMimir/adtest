use adtest::adtest;

#[adtest(
    setup = async async_setup_function
)]
fn test_with_setup() {
    println!("Do something")
}

fn setup_function() {
    println!("Doing setup");
}


#[adtest(setup = setup_function)]
async fn async_test_with_setup() {
    println!("Do something")
}

async fn async_setup_function() {
    println!("Doing setup");
}
