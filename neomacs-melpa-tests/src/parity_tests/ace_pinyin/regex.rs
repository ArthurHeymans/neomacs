use super::assert_ace_pinyin_parity;
use expect_test::expect;

#[test]
fn ace_pinyin_build_regexp_forwards_all_configuration_boolean_combinations() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (let ((ace-pinyin-enable-punctuation-translation
                  (nth 0 fixture))
                 (ace-pinyin-simplified-chinese-only-p
                  (nth 1 fixture))
                 (events nil))
             (cl-letf
                 (((symbol-function
                    'pinyinlib-build-regexp-char)
                   (lambda (query no-punctuation traditional prefix)
                     (push
                      (list query
                            no-punctuation
                            traditional
                            prefix)
                      events)
                     'regexp-result)))
               (list
                fixture
                (ace-pinyin--build-regexp
                 ?z
                 'fixture-prefix)
                (nreverse events)))))
         '((nil nil)
           (nil t)
           (t nil)
           (t t)))"##;
    let expect = expect![
        "OK (((nil nil) regexp-result ((122 t t fixture-prefix))) ((nil t) regexp-result ((122 t nil fixture-prefix))) ((t nil) regexp-result ((122 nil t fixture-prefix))) ((t t) regexp-result ((122 nil nil fixture-prefix))))"
    ];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_actual_simplified_and_traditional_character_regexps_match() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (let ((ace-pinyin-enable-punctuation-translation t)
                 (ace-pinyin-simplified-chinese-only-p
                  fixture))
             (list
              fixture
              (ace-pinyin--build-regexp ?z nil)
              (ace-pinyin--build-regexp ?z t))))
         '(t nil))"##;
    let expect = expect![[
        r#"OK ((t "[z杂扎砸咋咂匝拶在再载灾仔宰哉栽崽甾咱赞暂攒簪糌瓒拶昝趱錾藏脏葬赃臧锗奘驵早造遭糟澡灶躁噪凿枣皂燥蚤藻缲唣则责泽择咋啧仄迮笮箦舴帻赜昃贼怎谮增赠憎缯罾甑锃炸扎咋诈乍眨渣札栅轧闸榨喳揸柞楂哳吒铡砟齄咤痄蚱摘债宅窄斋寨翟砦瘵战展站占沾斩辗粘盏崭瞻绽蘸湛詹毡栈谵搌旃长张章丈掌涨帐障账胀仗杖彰璋蟑樟瘴漳嶂鄣獐仉幛嫜着找照招朝赵召罩兆昭肇沼诏钊啁棹笊这着者折哲浙遮辙辄谪蔗蛰褶鹧锗磔摺蜇赭柘真阵镇震针珍圳振诊枕斟贞侦赈甄臻箴疹砧桢缜畛轸胗稹祯浈溱蓁椹榛朕鸩政正证整争征挣郑症睁徵蒸怔筝拯铮峥狰诤鲭钲帧之只知至制直治指支志职致值织纸止质执智置址枝秩植旨滞徵帜稚挚汁掷殖芝吱肢脂峙侄窒蜘趾炙痔咫芷栉枳踯桎帙栀祉轾贽痣豸卮轵埴陟郅黹忮彘骘酯摭絷跖膣雉鸷胝蛭踬祗觯中种重众终钟忠衷肿仲锺踵盅冢忪舯螽周州洲粥舟皱骤轴宙咒昼肘帚胄纣诌绉妯碡啁荮籀繇酎主住注助著逐诸朱驻珠祝猪筑竹煮嘱柱烛铸株瞩蛛伫拄贮洙诛褚铢箸蛀茱炷躅竺杼翥渚潴麈槠橥苎侏瘃疰邾舳抓爪拽嘬传专转赚撰砖篆啭馔颛装状壮庄撞妆幢桩奘僮戆追坠缀锥赘隹椎惴骓缒准谆窀肫着桌捉卓琢灼酌拙浊濯茁啄斫镯涿焯浞倬禚诼擢子自字资咨紫滋仔姿吱兹孜梓渍籽姊恣滓谘龇秭呲辎锱眦笫髭淄茈觜訾缁耔鲻嵫赀孳粢趑总宗纵踪综棕粽鬃偬腙枞走奏邹揍驺鲰诹陬鄹组足族祖租阻卒诅俎镞菹赚钻攥纂躜缵最罪嘴醉咀觜蕞尊遵樽鳟撙作做坐座左昨琢佐凿撮柞嘬怍胙唑笮阼祚酢]" "[杂扎砸咋咂匝拶在再载灾仔宰哉栽崽甾咱赞暂攒簪糌瓒拶昝趱錾藏脏葬赃臧锗奘驵早造遭糟澡灶躁噪凿枣皂燥蚤藻缲唣则责泽择咋啧仄迮笮箦舴帻赜昃贼怎谮增赠憎缯罾甑锃炸扎咋诈乍眨渣札栅轧闸榨喳揸柞楂哳吒铡砟齄咤痄蚱摘债宅窄斋寨翟砦瘵战展站占沾斩辗粘盏崭瞻绽蘸湛詹毡栈谵搌旃长张章丈掌涨帐障账胀仗杖彰璋蟑樟瘴漳嶂鄣獐仉幛嫜着找照招朝赵召罩兆昭肇沼诏钊啁棹笊这着者折哲浙遮辙辄谪蔗蛰褶鹧锗磔摺蜇赭柘真阵镇震针珍圳振诊枕斟贞侦赈甄臻箴疹砧桢缜畛轸胗稹祯浈溱蓁椹榛朕鸩政正证整争征挣郑症睁徵蒸怔筝拯铮峥狰诤鲭钲帧之只知至制直治指支志职致值织纸止质执智置址枝秩植旨滞徵帜稚挚汁掷殖芝吱肢脂峙侄窒蜘趾炙痔咫芷栉枳踯桎帙栀祉轾贽痣豸卮轵埴陟郅黹忮彘骘酯摭絷跖膣雉鸷胝蛭踬祗觯中种重众终钟忠衷肿仲锺踵盅冢忪舯螽周州洲粥舟皱骤轴宙咒昼肘帚胄纣诌绉妯碡啁荮籀繇酎主住注助著逐诸朱驻珠祝猪筑竹煮嘱柱烛铸株瞩蛛伫拄贮洙诛褚铢箸蛀茱炷躅竺杼翥渚潴麈槠橥苎侏瘃疰邾舳抓爪拽嘬传专转赚撰砖篆啭馔颛装状壮庄撞妆幢桩奘僮戆追坠缀锥赘隹椎惴骓缒准谆窀肫着桌捉卓琢灼酌拙浊濯茁啄斫镯涿焯浞倬禚诼擢子自字资咨紫滋仔姿吱兹孜梓渍籽姊恣滓谘龇秭呲辎锱眦笫髭淄茈觜訾缁耔鲻嵫赀孳粢趑总宗纵踪综棕粽鬃偬腙枞走奏邹揍驺鲰诹陬鄹组足族祖租阻卒诅俎镞菹赚钻攥纂躜缵最罪嘴醉咀觜蕞尊遵樽鳟撙作做坐座左昨琢佐凿撮柞嘬怍胙唑笮阼祚酢]") (nil "[z雜扎砸咋咂匝拶在再載災仔宰哉栽崽甾咱贊暫攢簪糌瓚拶昝趲鏨藏髒葬贓臧鍺奘駔早造遭糟澡竈躁噪鑿棗皁燥蚤藻繰唣則責澤擇咋嘖仄迮笮簀舴幘賾昃賊怎譖增贈憎繒罾甑鋥炸扎咋詐乍眨渣札柵軋閘榨喳揸柞楂哳吒鍘砟齇吒痄蚱摘債宅窄齋寨翟砦瘵戰展站佔沾斬輾粘盞嶄瞻綻蘸湛詹氈棧譫搌旃長張章丈掌漲帳障賬脹仗杖彰璋蟑樟瘴漳嶂鄣獐仉幛嫜着找照招朝趙召罩兆昭肇沼詔釗啁棹笊這着者折哲浙遮轍輒謫蔗蟄褶鷓鍺磔摺蜇赭柘真陣鎮震針珍圳振診枕斟貞偵賑甄臻箴疹砧楨縝畛軫胗稹禎湞溱蓁椹榛朕鴆政正證整爭徵掙鄭症睜徵蒸怔箏拯錚崢猙諍鯖鉦幀之只知至制直治指支志職致值織紙止質執智置址枝秩植旨滯徵幟稚摯汁擲殖芝吱肢脂峙侄窒蜘趾炙痔咫芷櫛枳躑桎帙梔祉輊贄痣豸卮軹埴陟郅黹忮彘騭酯摭縶跖膣雉鷙胝蛭躓祗觶中種重衆終鍾忠衷腫仲鍾踵盅冢忪舯螽周州洲粥舟皺驟軸宙咒晝肘帚胄紂謅縐妯碡啁葤籀繇酎主住注助著逐諸朱駐珠祝豬築竹煮囑柱燭鑄株矚蛛佇拄貯洙誅褚銖箸蛀茱炷躅竺杼翥渚瀦麈櫧櫫苧侏瘃疰邾舳抓爪拽嘬傳專轉賺撰磚篆囀饌顓裝狀壯莊撞妝幢樁奘僮戇追墜綴錐贅隹椎惴騅縋準諄窀肫着桌捉卓琢灼酌拙濁濯茁啄斫鐲涿焯浞倬禚諑擢子自字資諮紫滋仔姿吱茲孜梓漬籽姊恣滓諮齜秭呲輜錙眥笫髭淄茈觜訾緇耔鯔嵫貲孳粢趑總宗縱蹤綜棕糉鬃傯腙樅走奏鄒揍騶鯫諏陬鄹組足族祖租阻卒詛俎鏃菹賺鑽攥纂躦纘最罪嘴醉咀觜蕞尊遵樽鱒撙作做坐座左昨琢佐鑿撮柞嘬怍胙唑笮阼祚酢]" "[雜扎砸咋咂匝拶在再載災仔宰哉栽崽甾咱贊暫攢簪糌瓚拶昝趲鏨藏髒葬贓臧鍺奘駔早造遭糟澡竈躁噪鑿棗皁燥蚤藻繰唣則責澤擇咋嘖仄迮笮簀舴幘賾昃賊怎譖增贈憎繒罾甑鋥炸扎咋詐乍眨渣札柵軋閘榨喳揸柞楂哳吒鍘砟齇吒痄蚱摘債宅窄齋寨翟砦瘵戰展站佔沾斬輾粘盞嶄瞻綻蘸湛詹氈棧譫搌旃長張章丈掌漲帳障賬脹仗杖彰璋蟑樟瘴漳嶂鄣獐仉幛嫜着找照招朝趙召罩兆昭肇沼詔釗啁棹笊這着者折哲浙遮轍輒謫蔗蟄褶鷓鍺磔摺蜇赭柘真陣鎮震針珍圳振診枕斟貞偵賑甄臻箴疹砧楨縝畛軫胗稹禎湞溱蓁椹榛朕鴆政正證整爭徵掙鄭症睜徵蒸怔箏拯錚崢猙諍鯖鉦幀之只知至制直治指支志職致值織紙止質執智置址枝秩植旨滯徵幟稚摯汁擲殖芝吱肢脂峙侄窒蜘趾炙痔咫芷櫛枳躑桎帙梔祉輊贄痣豸卮軹埴陟郅黹忮彘騭酯摭縶跖膣雉鷙胝蛭躓祗觶中種重衆終鍾忠衷腫仲鍾踵盅冢忪舯螽周州洲粥舟皺驟軸宙咒晝肘帚胄紂謅縐妯碡啁葤籀繇酎主住注助著逐諸朱駐珠祝豬築竹煮囑柱燭鑄株矚蛛佇拄貯洙誅褚銖箸蛀茱炷躅竺杼翥渚瀦麈櫧櫫苧侏瘃疰邾舳抓爪拽嘬傳專轉賺撰磚篆囀饌顓裝狀壯莊撞妝幢樁奘僮戇追墜綴錐贅隹椎惴騅縋準諄窀肫着桌捉卓琢灼酌拙濁濯茁啄斫鐲涿焯浞倬禚諑擢子自字資諮紫滋仔姿吱茲孜梓漬籽姊恣滓諮齜秭呲輜錙眥笫髭淄茈觜訾緇耔鯔嵫貲孳粢趑總宗縱蹤綜棕糉鬃傯腙樅走奏鄒揍騶鯫諏陬鄹組足族祖租阻卒詛俎鏃菹賺鑽攥纂躦纘最罪嘴醉咀觜蕞尊遵樽鱒撙作做坐座左昨琢佐鑿撮柞嘬怍胙唑笮阼祚酢]"))"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_actual_punctuation_translation_and_disable_switch_match() {
    let elisp_form = r##"(mapcar
         (lambda (enabled)
           (let ((ace-pinyin-enable-punctuation-translation
                  enabled)
                 (ace-pinyin-simplified-chinese-only-p t))
             (list
              enabled
              (ace-pinyin--build-regexp ?. nil)
              (ace-pinyin--build-regexp ?< nil))))
         '(t nil))"##;
    let expect = expect![[r#"OK ((t "[。.]" "[《<]") (nil "\\." "<"))"#]];
    assert_ace_pinyin_parity(elisp_form, expect);
}
