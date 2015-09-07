
use time::*;

use curve::*;
use animatable::*;
use pyramid::pon::*;
use cgmath::*;

#[derive(PartialEq, Debug, Clone)]
pub enum Loop {
    Forever,
    Once
}

#[derive(PartialEq, Debug, Clone)]
pub enum CurveTime {
    /// The curve is expected to have keys between 0 and 1
    Relative,
    /// The curve is expected to have keys between 0 and duration
    Absolute
}

#[derive(Debug)]
pub struct Animation {
    pub curve: Box<Curve<f32>>,
    pub offset: Duration,
    pub property: NamedPropRef,
    pub loop_type: Loop,
    pub duration: Duration,
    pub curve_time: CurveTime
}

impl Animation {
    pub fn new_fixed_value(property: NamedPropRef, value: f32) -> Animation {
        Animation {
            curve: Box::new(FixedValueCurve { value: value }),
            offset: Duration::zero(),
            property: property,
            loop_type: Loop::Forever,
            duration: Duration::weeks(1),
            curve_time: CurveTime::Absolute
        }
    }
}

impl Animatable for Animation {
    fn update(&self, time: Duration) -> Vec<(NamedPropRef, f32)> {
        let time = time - self.offset;
        let time = if time > self.duration {
            if self.loop_type == Loop::Forever {
                Duration::milliseconds(time.num_milliseconds() % self.duration.num_milliseconds())
            } else {
                return vec![]
            }
        } else {
            time
        };
        let time = match self.curve_time {
            CurveTime::Absolute => time.num_milliseconds() as f32 / 1000.0,
            CurveTime::Relative => time.num_milliseconds() as f32 / self.duration.num_milliseconds() as f32
        };
        return vec![(self.property.clone(), self.curve.value(time))];
    }
}


impl<'a> Translatable<'a, Loop> for Pon {
    fn inner_translate(&'a self) -> Result<Loop, PonTranslateErr> {
        match try!(self.translate()) {
            "forever" => Ok(Loop::Forever),
            "once" => Ok(Loop::Once),
            _ => Err(PonTranslateErr::InvalidValue { value: format!("{:?}", self) })
        }
    }
}
impl<'a> Translatable<'a, CurveTime> for Pon {
    fn inner_translate(&'a self) -> Result<CurveTime, PonTranslateErr> {
        match try!(self.translate()) {
            "absolute" => Ok(CurveTime::Absolute),
            "relative" => Ok(CurveTime::Relative),
            _ => Err(PonTranslateErr::InvalidValue { value: format!("{:?}", self) })
        }
    }
}

impl<'a> Translatable<'a, Key<f32>> for Pon {
    fn inner_translate(&'a self) -> Result<Key<f32>, PonTranslateErr> {
        match self {
            &Pon::Object(..) => {
                let time: f32 = try!(self.field_as::<f32>("time"));
                let value = try!(self.field_as("value"));
                Ok(Key(time, value))
            },
            &Pon::Array(ref arr) => {
                let time: f32 = try!(arr[0].translate::<f32>());
                let value = try!(arr[1].translate());
                Ok(Key(time, value))
            },
            &Pon::FloatArray(ref arr) => Ok(Key(arr[0], arr[1])),
            _ => {
                Err(PonTranslateErr::MismatchType { expected: "Object or Array".to_string(), found: format!("{:?}", self) })
            }
        }
    }
}

impl<'a> Translatable<'a, Animation> for Pon {
    fn inner_translate(&'a self) -> Result<Animation, PonTranslateErr> {
        let &TypedPon { ref type_name, ref data } = try!(self.translate());
        match type_name.as_str() {
            "key_framed" => {
                let property: &NamedPropRef = try!(try!(data.field("property")).as_reference());
                let duration: f32 = try!(data.field_as_or("duration", 1.0));
                let loop_type = try!(data.field_as_or("loop", Loop::Once));
                let curve_time = try!(data.field_as_or("curve_time", CurveTime::Absolute));
                let keys_array: &Vec<Pon> = try!(data.field_as("keys"));
                let first_key = &keys_array[0];
                let curve: Box<Curve<f32>> = {
                    let as_float: Result<Key<f32>, PonTranslateErr> = first_key.translate();
                    if let Ok(..) = as_float {
                        let keys: Vec<Key<f32>> = try!(data.field_as("keys"));
                        Box::new(LinearKeyFrameCurve {
                            keys: keys
                        })
                    } else {
                        return Err(PonTranslateErr::Generic(format!("Unrecognized keys: {:?}", first_key)))
                    }
                };
                Ok(Animation {
                    curve: curve,
                    offset: Duration::zero(),
                    property: property.clone(),
                    loop_type: loop_type,
                    duration: Duration::milliseconds((duration*1000.0) as i64),
                    curve_time: curve_time
                })
            },
            "fixed_value" => {
                let property: &NamedPropRef = try!(try!(data.field("property")).as_reference());
                let value = try!(data.field_as::<f32>("value"));
                Ok(Animation::new_fixed_value(property.clone(), value))
            },
            s @ _ => Err(PonTranslateErr::UnrecognizedType(s.to_string()))
        }
    }
}



#[test]
fn test_animation() {
    let kf = Animation {
        curve: Box::new(LinearKeyFrameCurve {
            keys: vec![Key(0.0, 0.0), Key(1.0, 1.0)]
        }),
        offset: Duration::zero(),
        property: NamedPropRef::new(EntityPath::This, "x"),
        loop_type: Loop::Once,
        duration: Duration::seconds(1),
        curve_time: CurveTime::Absolute
    };
    assert_eq!(kf.update(Duration::milliseconds(100)), vec![(NamedPropRef::new(EntityPath::This, "x"), 0.1)]);
    assert_eq!(kf.update(Duration::milliseconds(600)), vec![(NamedPropRef::new(EntityPath::This, "x"), 0.6)]);
}

#[test]
fn test_animation_from_pon() {
    let kf: Animation = Pon::from_string(
        "key_framed { property: this.x, keys: [{ time: 0.0, value: 0.0 }, { time: 1.0, value: 1.0 }], loop: 'forever' }").unwrap().translate().unwrap();
    assert_eq!(kf.update(Duration::milliseconds(100)), vec![(NamedPropRef::new(EntityPath::This, "x"), 0.1)]);
    assert_eq!(kf.update(Duration::milliseconds(600)), vec![(NamedPropRef::new(EntityPath::This, "x"), 0.6)]);
}

#[test]
fn test_animation_from_pon_alternative_syntax() {
    let kf: Animation = Pon::from_string(
        "key_framed { property: this.x, keys: [[0.0, 0.0], { time: 1.0, value: 1.0 }], loop: 'forever' }").unwrap().translate().unwrap();
    assert_eq!(kf.update(Duration::milliseconds(100)), vec![(NamedPropRef::new(EntityPath::This, "x"), 0.1)]);
    assert_eq!(kf.update(Duration::milliseconds(600)), vec![(NamedPropRef::new(EntityPath::This, "x"), 0.6)]);
}
