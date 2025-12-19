from django.urls import path, include
from . import views

urlpatterns = [
    path("members", views.get_members),
    path("members/<int:member_id>", views.get_member),
    path("members/new", views.create_member),
]