
use time::*;
use pyramid::pon::*;
use curve_track::*;
use track_set::*;
use weighted_tracks::*;
use animatable::*;
use std::fmt::Debug;
use std::rc::Rc;

pub trait Track : Debug {
    fn value_at(&self, time: Duration) -> Vec<(NamedPropRef, Animatable)>;
}

#[derive(Debug)]
struct TrackSetFromResource {
    resource: Rc<TrackSet>
}

impl Track for TrackSetFromResource {
    fn value_at(&self, time: Duration) -> Vec<(NamedPropRef, Animatable)> {
        self.resource.value_at(time)
    }
}

impl Translatable<Box<Track>> for Pon {
    fn inner_translate(&self, context: &mut TranslateContext) -> Result<Box<Track>, PonTranslateErr> {
        self.as_typed(|&TypedPon { ref type_name, ref data }| -> Result<Box<Track>, PonTranslateErr> {
            match type_name.as_str() {
                "key_framed" => Ok(Box::new(try!(self.translate::<CurveTrack>(context)))),
                "fixed_value" => Ok(Box::new(try!(self.translate::<CurveTrack>(context)))),
                "track_set" => Ok(Box::new(try!(self.translate::<TrackSet>(context)))),
                "weighted_tracks" => Ok(Box::new(try!(self.translate::<WeightedTracks>(context)))),
                "track_set_from_resource" => {
                    let resource_id = try!(data.translate::<String>(context));
                    let track_set = context.document.unwrap().resources.get(&resource_id).unwrap().downcast_ref::<Rc<TrackSet>>().unwrap().clone();
                    return Ok(Box::new(TrackSetFromResource { resource: track_set }));
                },
                s @ _ => Err(PonTranslateErr::UnrecognizedType(s.to_string()))
            }
        })
    }
}
