_sealtask() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="sealtask"
                ;;
            sealtask,auth)
                cmd="sealtask__subcmd__auth"
                ;;
            sealtask,comments)
                cmd="sealtask__subcmd__comments"
                ;;
            sealtask,completion)
                cmd="sealtask__subcmd__completion"
                ;;
            sealtask,config)
                cmd="sealtask__subcmd__config"
                ;;
            sealtask,doctor)
                cmd="sealtask__subcmd__doctor"
                ;;
            sealtask,help)
                cmd="sealtask__subcmd__help"
                ;;
            sealtask,info)
                cmd="sealtask__subcmd__info"
                ;;
            sealtask,inspect)
                cmd="sealtask__subcmd__inspect"
                ;;
            sealtask,lists)
                cmd="sealtask__subcmd__projects"
                ;;
            sealtask,man)
                cmd="sealtask__subcmd__man"
                ;;
            sealtask,me)
                cmd="sealtask__subcmd__me"
                ;;
            sealtask,notes)
                cmd="sealtask__subcmd__notes"
                ;;
            sealtask,profile)
                cmd="sealtask__subcmd__profile"
                ;;
            sealtask,projects)
                cmd="sealtask__subcmd__projects"
                ;;
            sealtask,schema)
                cmd="sealtask__subcmd__schema"
                ;;
            sealtask,stats)
                cmd="sealtask__subcmd__stats"
                ;;
            sealtask,tasks)
                cmd="sealtask__subcmd__tasks"
                ;;
            sealtask__subcmd__auth,help)
                cmd="sealtask__subcmd__auth__subcmd__help"
                ;;
            sealtask__subcmd__auth,keychain)
                cmd="sealtask__subcmd__auth__subcmd__keychain"
                ;;
            sealtask__subcmd__auth,lock)
                cmd="sealtask__subcmd__auth__subcmd__lock"
                ;;
            sealtask__subcmd__auth,login)
                cmd="sealtask__subcmd__auth__subcmd__login"
                ;;
            sealtask__subcmd__auth,logout)
                cmd="sealtask__subcmd__auth__subcmd__logout"
                ;;
            sealtask__subcmd__auth,status)
                cmd="sealtask__subcmd__auth__subcmd__status"
                ;;
            sealtask__subcmd__auth,unlock)
                cmd="sealtask__subcmd__auth__subcmd__unlock"
                ;;
            sealtask__subcmd__auth__subcmd__help,help)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__auth__subcmd__help,keychain)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__keychain"
                ;;
            sealtask__subcmd__auth__subcmd__help,lock)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__lock"
                ;;
            sealtask__subcmd__auth__subcmd__help,login)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__login"
                ;;
            sealtask__subcmd__auth__subcmd__help,logout)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__logout"
                ;;
            sealtask__subcmd__auth__subcmd__help,status)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__status"
                ;;
            sealtask__subcmd__auth__subcmd__help,unlock)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__unlock"
                ;;
            sealtask__subcmd__auth__subcmd__help__subcmd__keychain,clear)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__keychain__subcmd__clear"
                ;;
            sealtask__subcmd__auth__subcmd__help__subcmd__keychain,store)
                cmd="sealtask__subcmd__auth__subcmd__help__subcmd__keychain__subcmd__store"
                ;;
            sealtask__subcmd__auth__subcmd__keychain,clear)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__clear"
                ;;
            sealtask__subcmd__auth__subcmd__keychain,help)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__help"
                ;;
            sealtask__subcmd__auth__subcmd__keychain,store)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__store"
                ;;
            sealtask__subcmd__auth__subcmd__keychain__subcmd__help,clear)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__clear"
                ;;
            sealtask__subcmd__auth__subcmd__keychain__subcmd__help,help)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__auth__subcmd__keychain__subcmd__help,store)
                cmd="sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__store"
                ;;
            sealtask__subcmd__comments,create)
                cmd="sealtask__subcmd__comments__subcmd__create"
                ;;
            sealtask__subcmd__comments,delete)
                cmd="sealtask__subcmd__comments__subcmd__delete"
                ;;
            sealtask__subcmd__comments,help)
                cmd="sealtask__subcmd__comments__subcmd__help"
                ;;
            sealtask__subcmd__comments,list)
                cmd="sealtask__subcmd__comments__subcmd__list"
                ;;
            sealtask__subcmd__comments,update)
                cmd="sealtask__subcmd__comments__subcmd__update"
                ;;
            sealtask__subcmd__comments__subcmd__help,create)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__create"
                ;;
            sealtask__subcmd__comments__subcmd__help,delete)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__delete"
                ;;
            sealtask__subcmd__comments__subcmd__help,help)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__comments__subcmd__help,list)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__comments__subcmd__help,update)
                cmd="sealtask__subcmd__comments__subcmd__help__subcmd__update"
                ;;
            sealtask__subcmd__config,help)
                cmd="sealtask__subcmd__config__subcmd__help"
                ;;
            sealtask__subcmd__config,show)
                cmd="sealtask__subcmd__config__subcmd__show"
                ;;
            sealtask__subcmd__config__subcmd__help,help)
                cmd="sealtask__subcmd__config__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__config__subcmd__help,show)
                cmd="sealtask__subcmd__config__subcmd__help__subcmd__show"
                ;;
            sealtask__subcmd__help,auth)
                cmd="sealtask__subcmd__help__subcmd__auth"
                ;;
            sealtask__subcmd__help,comments)
                cmd="sealtask__subcmd__help__subcmd__comments"
                ;;
            sealtask__subcmd__help,completion)
                cmd="sealtask__subcmd__help__subcmd__completion"
                ;;
            sealtask__subcmd__help,config)
                cmd="sealtask__subcmd__help__subcmd__config"
                ;;
            sealtask__subcmd__help,doctor)
                cmd="sealtask__subcmd__help__subcmd__doctor"
                ;;
            sealtask__subcmd__help,help)
                cmd="sealtask__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__help,info)
                cmd="sealtask__subcmd__help__subcmd__info"
                ;;
            sealtask__subcmd__help,inspect)
                cmd="sealtask__subcmd__help__subcmd__inspect"
                ;;
            sealtask__subcmd__help,man)
                cmd="sealtask__subcmd__help__subcmd__man"
                ;;
            sealtask__subcmd__help,me)
                cmd="sealtask__subcmd__help__subcmd__me"
                ;;
            sealtask__subcmd__help,notes)
                cmd="sealtask__subcmd__help__subcmd__notes"
                ;;
            sealtask__subcmd__help,profile)
                cmd="sealtask__subcmd__help__subcmd__profile"
                ;;
            sealtask__subcmd__help,projects)
                cmd="sealtask__subcmd__help__subcmd__projects"
                ;;
            sealtask__subcmd__help,schema)
                cmd="sealtask__subcmd__help__subcmd__schema"
                ;;
            sealtask__subcmd__help,stats)
                cmd="sealtask__subcmd__help__subcmd__stats"
                ;;
            sealtask__subcmd__help,tasks)
                cmd="sealtask__subcmd__help__subcmd__tasks"
                ;;
            sealtask__subcmd__help__subcmd__auth,keychain)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__keychain"
                ;;
            sealtask__subcmd__help__subcmd__auth,lock)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__lock"
                ;;
            sealtask__subcmd__help__subcmd__auth,login)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__login"
                ;;
            sealtask__subcmd__help__subcmd__auth,logout)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__logout"
                ;;
            sealtask__subcmd__help__subcmd__auth,status)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__status"
                ;;
            sealtask__subcmd__help__subcmd__auth,unlock)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__unlock"
                ;;
            sealtask__subcmd__help__subcmd__auth__subcmd__keychain,clear)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__keychain__subcmd__clear"
                ;;
            sealtask__subcmd__help__subcmd__auth__subcmd__keychain,store)
                cmd="sealtask__subcmd__help__subcmd__auth__subcmd__keychain__subcmd__store"
                ;;
            sealtask__subcmd__help__subcmd__comments,create)
                cmd="sealtask__subcmd__help__subcmd__comments__subcmd__create"
                ;;
            sealtask__subcmd__help__subcmd__comments,delete)
                cmd="sealtask__subcmd__help__subcmd__comments__subcmd__delete"
                ;;
            sealtask__subcmd__help__subcmd__comments,list)
                cmd="sealtask__subcmd__help__subcmd__comments__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__comments,update)
                cmd="sealtask__subcmd__help__subcmd__comments__subcmd__update"
                ;;
            sealtask__subcmd__help__subcmd__config,show)
                cmd="sealtask__subcmd__help__subcmd__config__subcmd__show"
                ;;
            sealtask__subcmd__help__subcmd__notes,create)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__create"
                ;;
            sealtask__subcmd__help__subcmd__notes,delete)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__delete"
                ;;
            sealtask__subcmd__help__subcmd__notes,get)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__get"
                ;;
            sealtask__subcmd__help__subcmd__notes,list)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__notes,update)
                cmd="sealtask__subcmd__help__subcmd__notes__subcmd__update"
                ;;
            sealtask__subcmd__help__subcmd__profile,list)
                cmd="sealtask__subcmd__help__subcmd__profile__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__profile,use)
                cmd="sealtask__subcmd__help__subcmd__profile__subcmd__use"
                ;;
            sealtask__subcmd__help__subcmd__projects,archive)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__archive"
                ;;
            sealtask__subcmd__help__subcmd__projects,clear)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__clear"
                ;;
            sealtask__subcmd__help__subcmd__projects,current)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__current"
                ;;
            sealtask__subcmd__help__subcmd__projects,get)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__get"
                ;;
            sealtask__subcmd__help__subcmd__projects,list)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__projects,sections)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__sections"
                ;;
            sealtask__subcmd__help__subcmd__projects,unarchive)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__unarchive"
                ;;
            sealtask__subcmd__help__subcmd__projects,use)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__use"
                ;;
            sealtask__subcmd__help__subcmd__projects__subcmd__sections,list)
                cmd="sealtask__subcmd__help__subcmd__projects__subcmd__sections__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__tasks,archive)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__archive"
                ;;
            sealtask__subcmd__help__subcmd__tasks,attachments)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments"
                ;;
            sealtask__subcmd__help__subcmd__tasks,complete)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__complete"
                ;;
            sealtask__subcmd__help__subcmd__tasks,create)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__create"
                ;;
            sealtask__subcmd__help__subcmd__tasks,delete)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__delete"
                ;;
            sealtask__subcmd__help__subcmd__tasks,get)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__get"
                ;;
            sealtask__subcmd__help__subcmd__tasks,list)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__list"
                ;;
            sealtask__subcmd__help__subcmd__tasks,move)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__move"
                ;;
            sealtask__subcmd__help__subcmd__tasks,reopen)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__reopen"
                ;;
            sealtask__subcmd__help__subcmd__tasks,unarchive)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__unarchive"
                ;;
            sealtask__subcmd__help__subcmd__tasks,update)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__update"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__attachments,delete)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__delete"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__attachments,download)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__download"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__attachments,read)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__read"
                ;;
            sealtask__subcmd__help__subcmd__tasks__subcmd__attachments,upload)
                cmd="sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__upload"
                ;;
            sealtask__subcmd__notes,create)
                cmd="sealtask__subcmd__notes__subcmd__create"
                ;;
            sealtask__subcmd__notes,delete)
                cmd="sealtask__subcmd__notes__subcmd__delete"
                ;;
            sealtask__subcmd__notes,get)
                cmd="sealtask__subcmd__notes__subcmd__get"
                ;;
            sealtask__subcmd__notes,help)
                cmd="sealtask__subcmd__notes__subcmd__help"
                ;;
            sealtask__subcmd__notes,list)
                cmd="sealtask__subcmd__notes__subcmd__list"
                ;;
            sealtask__subcmd__notes,update)
                cmd="sealtask__subcmd__notes__subcmd__update"
                ;;
            sealtask__subcmd__notes__subcmd__help,create)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__create"
                ;;
            sealtask__subcmd__notes__subcmd__help,delete)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__delete"
                ;;
            sealtask__subcmd__notes__subcmd__help,get)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__get"
                ;;
            sealtask__subcmd__notes__subcmd__help,help)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__notes__subcmd__help,list)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__notes__subcmd__help,update)
                cmd="sealtask__subcmd__notes__subcmd__help__subcmd__update"
                ;;
            sealtask__subcmd__profile,help)
                cmd="sealtask__subcmd__profile__subcmd__help"
                ;;
            sealtask__subcmd__profile,list)
                cmd="sealtask__subcmd__profile__subcmd__list"
                ;;
            sealtask__subcmd__profile,use)
                cmd="sealtask__subcmd__profile__subcmd__use"
                ;;
            sealtask__subcmd__profile__subcmd__help,help)
                cmd="sealtask__subcmd__profile__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__profile__subcmd__help,list)
                cmd="sealtask__subcmd__profile__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__profile__subcmd__help,use)
                cmd="sealtask__subcmd__profile__subcmd__help__subcmd__use"
                ;;
            sealtask__subcmd__projects,archive)
                cmd="sealtask__subcmd__projects__subcmd__archive"
                ;;
            sealtask__subcmd__projects,clear)
                cmd="sealtask__subcmd__projects__subcmd__clear"
                ;;
            sealtask__subcmd__projects,current)
                cmd="sealtask__subcmd__projects__subcmd__current"
                ;;
            sealtask__subcmd__projects,get)
                cmd="sealtask__subcmd__projects__subcmd__get"
                ;;
            sealtask__subcmd__projects,help)
                cmd="sealtask__subcmd__projects__subcmd__help"
                ;;
            sealtask__subcmd__projects,list)
                cmd="sealtask__subcmd__projects__subcmd__list"
                ;;
            sealtask__subcmd__projects,sections)
                cmd="sealtask__subcmd__projects__subcmd__sections"
                ;;
            sealtask__subcmd__projects,unarchive)
                cmd="sealtask__subcmd__projects__subcmd__unarchive"
                ;;
            sealtask__subcmd__projects,use)
                cmd="sealtask__subcmd__projects__subcmd__use"
                ;;
            sealtask__subcmd__projects__subcmd__help,archive)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__archive"
                ;;
            sealtask__subcmd__projects__subcmd__help,clear)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__clear"
                ;;
            sealtask__subcmd__projects__subcmd__help,current)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__current"
                ;;
            sealtask__subcmd__projects__subcmd__help,get)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__get"
                ;;
            sealtask__subcmd__projects__subcmd__help,help)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__projects__subcmd__help,list)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__projects__subcmd__help,sections)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__sections"
                ;;
            sealtask__subcmd__projects__subcmd__help,unarchive)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__unarchive"
                ;;
            sealtask__subcmd__projects__subcmd__help,use)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__use"
                ;;
            sealtask__subcmd__projects__subcmd__help__subcmd__sections,list)
                cmd="sealtask__subcmd__projects__subcmd__help__subcmd__sections__subcmd__list"
                ;;
            sealtask__subcmd__projects__subcmd__sections,help)
                cmd="sealtask__subcmd__projects__subcmd__sections__subcmd__help"
                ;;
            sealtask__subcmd__projects__subcmd__sections,list)
                cmd="sealtask__subcmd__projects__subcmd__sections__subcmd__list"
                ;;
            sealtask__subcmd__projects__subcmd__sections__subcmd__help,help)
                cmd="sealtask__subcmd__projects__subcmd__sections__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__projects__subcmd__sections__subcmd__help,list)
                cmd="sealtask__subcmd__projects__subcmd__sections__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__tasks,archive)
                cmd="sealtask__subcmd__tasks__subcmd__archive"
                ;;
            sealtask__subcmd__tasks,attachments)
                cmd="sealtask__subcmd__tasks__subcmd__attachments"
                ;;
            sealtask__subcmd__tasks,complete)
                cmd="sealtask__subcmd__tasks__subcmd__complete"
                ;;
            sealtask__subcmd__tasks,create)
                cmd="sealtask__subcmd__tasks__subcmd__create"
                ;;
            sealtask__subcmd__tasks,delete)
                cmd="sealtask__subcmd__tasks__subcmd__delete"
                ;;
            sealtask__subcmd__tasks,get)
                cmd="sealtask__subcmd__tasks__subcmd__get"
                ;;
            sealtask__subcmd__tasks,help)
                cmd="sealtask__subcmd__tasks__subcmd__help"
                ;;
            sealtask__subcmd__tasks,list)
                cmd="sealtask__subcmd__tasks__subcmd__list"
                ;;
            sealtask__subcmd__tasks,move)
                cmd="sealtask__subcmd__tasks__subcmd__move"
                ;;
            sealtask__subcmd__tasks,reopen)
                cmd="sealtask__subcmd__tasks__subcmd__reopen"
                ;;
            sealtask__subcmd__tasks,unarchive)
                cmd="sealtask__subcmd__tasks__subcmd__unarchive"
                ;;
            sealtask__subcmd__tasks,update)
                cmd="sealtask__subcmd__tasks__subcmd__update"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,delete)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__delete"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,download)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__download"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,help)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,read)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__read"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments,upload)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__upload"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,delete)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__delete"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,download)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__download"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,help)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,read)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__read"
                ;;
            sealtask__subcmd__tasks__subcmd__attachments__subcmd__help,upload)
                cmd="sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__upload"
                ;;
            sealtask__subcmd__tasks__subcmd__help,archive)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__archive"
                ;;
            sealtask__subcmd__tasks__subcmd__help,attachments)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments"
                ;;
            sealtask__subcmd__tasks__subcmd__help,complete)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__complete"
                ;;
            sealtask__subcmd__tasks__subcmd__help,create)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__create"
                ;;
            sealtask__subcmd__tasks__subcmd__help,delete)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__delete"
                ;;
            sealtask__subcmd__tasks__subcmd__help,get)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__get"
                ;;
            sealtask__subcmd__tasks__subcmd__help,help)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__help"
                ;;
            sealtask__subcmd__tasks__subcmd__help,list)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__list"
                ;;
            sealtask__subcmd__tasks__subcmd__help,move)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__move"
                ;;
            sealtask__subcmd__tasks__subcmd__help,reopen)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__reopen"
                ;;
            sealtask__subcmd__tasks__subcmd__help,unarchive)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__unarchive"
                ;;
            sealtask__subcmd__tasks__subcmd__help,update)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__update"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__attachments,delete)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__delete"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__attachments,download)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__download"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__attachments,read)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__read"
                ;;
            sealtask__subcmd__tasks__subcmd__help__subcmd__attachments,upload)
                cmd="sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__upload"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        sealtask)
            opts="-v -h -V --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --serve-unlock-daemon --help --version completion man info schema auth me projects lists tasks stats doctor config profile inspect comments notes help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --serve-unlock-daemon)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help login unlock lock keychain logout status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help)
            opts="login unlock lock keychain logout status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__keychain)
            opts="store clear"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__keychain__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__keychain__subcmd__store)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__lock)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__login)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__logout)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__help__subcmd__unlock)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help store clear help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__clear)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__help)
            opts="store clear help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__help__subcmd__store)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__keychain__subcmd__store)
            opts="-v -h --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__lock)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__login)
            opts="-v -h --email --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --email)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__logout)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__status)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__auth__subcmd__unlock)
            opts="-v -h --ttl --ttl-seconds --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ttl)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ttl-seconds)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help list create update delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__create)
            opts="-v -h --task-id --project --work-list-id --body --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__delete)
            opts="-v -h --task-id --project --work-list-id --comment-id --input-file --input-stdin --password-stdin --yes --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --comment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help)
            opts="list create update delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__list)
            opts="-v -h --project --work-list-id --task-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__comments__subcmd__update)
            opts="-v -h --task-id --project --work-list-id --comment-id --body --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --comment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__completion)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help bash zsh fish powershell"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help show help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config__subcmd__help)
            opts="show help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config__subcmd__help__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__config__subcmd__show)
            opts="-v -h --resolved --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__doctor)
            opts="-v -h --offline --strict --include-keychain --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help)
            opts="completion man info schema auth me projects tasks stats doctor config profile inspect comments notes help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth)
            opts="login unlock lock keychain logout status"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__keychain)
            opts="store clear"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__keychain__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__keychain__subcmd__store)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__lock)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__login)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__logout)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__auth__subcmd__unlock)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments)
            opts="list create update delete"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__comments__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__completion)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__config)
            opts="show"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__config__subcmd__show)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__doctor)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__inspect)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__man)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__me)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes)
            opts="list get create update delete"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__notes__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__profile)
            opts="list use"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__profile__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__profile__subcmd__use)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects)
            opts="list get archive unarchive use current clear sections"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__archive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__current)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__sections)
            opts="list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__sections__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__unarchive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__projects__subcmd__use)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__schema)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks)
            opts="list get create update move complete reopen archive unarchive delete attachments"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__archive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments)
            opts="upload delete read download"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__download)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__read)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__attachments__subcmd__upload)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__complete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__move)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__reopen)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__unarchive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__help__subcmd__tasks__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__info)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__inspect)
            opts="-v -h --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__man)
            opts="-v -h --output-dir --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__me)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help list get create update delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__create)
            opts="-v -h --project --work-list-id --title --body --private --idempotency-key --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --idempotency-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__delete)
            opts="-v -h --note-id --project --work-list-id --input-file --input-stdin --password-stdin --yes --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --note-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__get)
            opts="-v -h --note-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --note-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help)
            opts="list get create update delete help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__list)
            opts="-v -h --project --work-list-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__notes__subcmd__update)
            opts="-v -h --note-id --project --work-list-id --title --body --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --note-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help list use help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__help)
            opts="list use help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__help__subcmd__use)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__list)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__profile__subcmd__use)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects)
            opts="-v -h --verbose --include-archived --password-stdin --raw --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help list get archive unarchive use current clear sections help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__archive)
            opts="-v -h --password-stdin --raw --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__clear)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__current)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__get)
            opts="-v -h --password-stdin --raw --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help)
            opts="list get archive unarchive use current clear sections help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__archive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__current)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__sections)
            opts="list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__sections__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__unarchive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__help__subcmd__use)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__list)
            opts="-v -h --verbose --include-archived --password-stdin --raw --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections__subcmd__help)
            opts="list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__sections__subcmd__list)
            opts="-v -h --project --work-list-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__unarchive)
            opts="-v -h --password-stdin --raw --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__projects__subcmd__use)
            opts="-v -h --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__schema)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__stats)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help list get create update move complete reopen archive unarchive delete attachments help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__archive)
            opts="-v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments)
            opts="-v -h --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help upload delete read download help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__delete)
            opts="-v -h --task-id --project --work-list-id --attachment-id --password-stdin --yes --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attachment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__download)
            opts="-v -h --task-id --project --work-list-id --attachment-id --output --force --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attachment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help)
            opts="upload delete read download help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__download)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__read)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__help__subcmd__upload)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__read)
            opts="-v -h --task-id --project --work-list-id --attachment-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --attachment-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__attachments__subcmd__upload)
            opts="-v -h --task-id --project --work-list-id --file --file-name --content-type --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file-name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --content-type)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__complete)
            opts="-v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__create)
            opts="-v -h --project --work-list-id --title --body --priority --due-at --due --start-at --section-id --section --idempotency-key --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --priority)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --due-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --due)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --start-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --idempotency-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__delete)
            opts="-v -h --task-id --project --work-list-id --input-file --input-stdin --password-stdin --yes --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__get)
            opts="-v -h --task-id --project --work-list-id --password-stdin --raw --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help)
            opts="list get create update move complete reopen archive unarchive delete attachments help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__archive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments)
            opts="upload delete read download"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__download)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__read)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__attachments__subcmd__upload)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__complete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__move)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__reopen)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__unarchive)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__list)
            opts="-v -h --project --work-list-id --include-completed --include-archived --all --password-stdin --raw --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__move)
            opts="-v -h --task-id --project --work-list-id --section-id --section --insert-before-task-id --before --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --insert-before-task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --before)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__reopen)
            opts="-v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__unarchive)
            opts="-v -h --task-id --project --work-list-id --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        sealtask__subcmd__tasks__subcmd__update)
            opts="-v -h --task-id --project --work-list-id --title --body --clear-body --priority --clear-priority --due-at --due --clear-due-at --start-at --clear-start-at --section-id --section --clear-section --input-file --input-stdin --password-stdin --api-url --storage-origin --json --format --non-interactive --debug --connect-timeout --read-timeout --request-timeout --profile --config-dir --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --task-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --project)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --work-list-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --title)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --body)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --priority)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --due-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --due)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --start-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --section)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --input-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --storage-origin)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "table json json-pretty" -- "${cur}"))
                    return 0
                    ;;
                --connect-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --read-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --request-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _sealtask -o nosort -o bashdefault -o default sealtask
else
    complete -F _sealtask -o bashdefault -o default sealtask
fi
