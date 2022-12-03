

#![feature( try_blocks, option_result_contains, is_some_and, if_let_guard, let_chains, default_free_fn, box_syntax, async_closure )]
#![allow( unused_attributes, unused_variables, unused_imports, unused_assignments, unused_mut, unused_must_use, unreachable_code, path_statements )]

use std::{ default::{ default }, pin::*, future::*, task::*, sync::Arc, rc::*, cell::* };
use wgpu::{ * };
use winit::{ event_loop::{ * }, window::{ * }, error::{ * }, event::{ * } };
use waker_fn::waker_fn;
use parking_lot::{ Mutex };


fn main() {

	// render: present in window through a camera
	// terrain: (x,y,z) => SpaceState; where SpaceState is material, vectors, ...
	// render( terrain );

	#[derive(Default, Debug, Clone)]
	struct FutureExit ( Arc<Mutex<( bool, Option<Waker> )>> );

		impl Future for FutureExit {
			type Output = ();
			fn poll( self: Pin<&mut Self>, context: &mut Context<'_> ) -> Poll<<Self as Future>::Output> {
				let ( done, maybe_waker ) = &mut *self.0.lock();
				if *done {
					Poll::Ready(())
					}
				else {
					*maybe_waker = Some(context.waker().clone());
					Poll::Pending
					} 
				}
			}

	#[derive(Default, Debug, Clone)]
	struct FutureWindow ( Arc<Mutex<( Option<Waker>, Option<Arc<Mutex<Result<Window, OsError>>>> )>> );

		impl Future for FutureWindow {		
			type Output = Arc<Mutex<Result<Window, OsError>>>;
			
			fn poll( self: Pin<&mut Self>, context: &mut Context<'_> ) -> Poll<<Self as Future>::Output> { 
				let ( maybe_waker, maybe_window ) = &mut *self.0.lock();
				if let Some(maybe_window) = maybe_window {
					Poll::Ready(Arc::clone(maybe_window))	
					}
				else {
					*maybe_waker = Some(context.waker().clone());
					Poll::Pending
					}
				}
			}

	type BuildWindowResult = FutureWindow;

	#[derive(Debug)]
	enum UserEvent {
		PollMain,
		BuildWindow( BuildWindowResult, WindowBuilder ),
		}

  let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
  // state of EventLoop's callback
  let event_sender = event_loop.create_proxy();
  let window: Rc<RefCell<Option<Window>>> = default();
  let future_exit: FutureExit = default();
  
  let mut future: Pin<Box<_>> = Box::pin( {

  	let window = Rc::clone( &window );
  	let event_sender = event_sender.clone();
  	let future_exit = future_exit.clone();

  	async move {
  		println!("Main running");

  		let future_window: BuildWindowResult = default();
  		event_sender.send_event(UserEvent::BuildWindow( future_window.clone(), WindowBuilder::new()
  			.with_title("Terrain Render")
      	// .with_inner_size(LogicalSize::new(128.0, 128.0)))
      	) );

  		*window.borrow_mut() = Some(Arc::try_unwrap(future_window.await).unwrap().into_inner().unwrap());
  		// let window = future_window.await; // Arc


  		println!("Main with window");

  		future_exit.await;

  		println!("Main done");
  
     	// let window = window.as_ref().unwrap();
      // let size = window.inner_size();
  
      // let instance = Instance::new( Backends::all() );
      // let surface = unsafe { instance.create_surface(window).unwrap() };
      
      // let f = async { 
  
      // 	let adapter = instance.request_adapter( &RequestAdapterOptions {
      //     power_preference: default(),
      //     compatible_surface: Some(&surface),
      //     force_fallback_adapter: false,
      //     } ).await.unwrap(); 
  
      // 	};
    	} } );

  println!("run Loop");
  // all resources owned by this
  event_loop.run( move | event, event_loop, control_flow | match event {

  	Event::NewEvents(StartCause::Init) => {
  		println!("init Main");
  		event_sender.send_event(UserEvent::PollMain).unwrap();
      },

    Event::UserEvent( user_event ) => match user_event {

    	UserEvent::PollMain => {
    		println!("poll Main");
    		let event_sender = Mutex::new(event_sender.clone());
				let waker = waker_fn( move || event_sender.lock().send_event(UserEvent::PollMain).unwrap() );
				let mut context = Context::from_waker( &waker );

				if let Poll::Ready(_) = future.as_mut().poll( &mut context ) {
					*control_flow = ControlFlow::Exit;
					println!("exiting");
					}

				},

			UserEvent::BuildWindow( mut future_window, window_builder ) => {
				println!("build Window");
				let result = window_builder.build( &event_loop );
				let ( maybe_waker, maybe_window ) = &mut *future_window.0.lock();

				*maybe_window = Some(Arc::new(Mutex::new(result)));
				
				if let Some(waker) = maybe_waker.take() {
					waker.wake();
					}

				println!("build Window done");
				},

    	},

    Event::WindowEvent { ref event, window_id, } 
    if let Some( ref window ) = *window.borrow() && window.id() == window_id => match event {
     	
      WindowEvent::CloseRequested
      | WindowEvent::KeyboardInput { input: KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. }, .. } => {
      	println!("input Exit");
    		let ( done, maybe_waker ) = &mut *future_exit.0.lock();

    		*done = true;

      	if let Some(waker) = maybe_waker.take() {

					waker.wake();
					}
      	},
       
      _ => { 
      	// println!("some WindowEvent");
      	} },
    
    event => { 
    	if let Event::WindowEvent { event, window_id } = event {
    		println!("{:?}, {:?}", window_id, (*window.borrow()).as_ref().and_then( | window | Some(window.id()) ) );
    		}
    	// println!("some Event {:?}", event );
    	// *control_flow = ControlFlow::Wait;
    	} } ); }



